use std::io::{BufRead, Write};

use borsh::{BorshDeserialize, BorshSerialize};

use crate::bitcoin::encoding::{
    decode::MAX_VEC_SIZE, extensions::WriteExt, utils::VarInt, Decodable, Encodable,
};

/// The Witness is the data used to unlock bitcoin since the [segwit upgrade].
///
/// Can be logically seen as an array of bytestrings, i.e. `Vec<Vec<u8>>`, and it is serialized on the wire
/// in that format. You can convert between this type and `Vec<Vec<u8>>` by using [`Witness::from_slice`]
/// and [`Witness::to_vec`].
///
/// For serialization and deserialization performance it is stored internally as a single `Vec`,
/// saving some allocations.
///
/// [segwit upgrade]: <https://github.com/bitcoin/bips/blob/master/bip-0143.mediawiki>
#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct Witness {
    /// Contains the witness `Vec<Vec<u8>>` serialization.
    ///
    /// Does not include the initial varint indicating the number of elements. Each element however,
    /// does include a varint indicating the element length. The number of elements is stored in
    /// `witness_elements`.
    ///
    /// Concatenated onto the end of `content` is the index area. This is a `4 * witness_elements`
    /// bytes area which stores the index of the start of each witness item.
    content: Vec<u8>,

    /// The number of elements in the witness.
    ///
    /// Stored separately (instead of as a VarInt in the initial part of content) so that methods
    /// like [`Witness::push`] don't have to shift the entire array.
    witness_elements: usize,

    /// This is the valid index pointing to the beginning of the index area.
    ///
    /// Said another way, this is the total length of all witness elements serialized (without the
    /// element count but with their sizes serialized as compact size).
    indices_start: usize,
}

impl Default for Witness {
    fn default() -> Self {
        Self::new()
    }
}

impl Witness {
    pub const fn new() -> Self {
        Self {
            content: Vec::new(),
            witness_elements: 0,
            indices_start: 0,
        }
    }

    /// Returns the number of elements this witness holds.
    pub const fn len(&self) -> usize {
        self.witness_elements
    }

    /// Returns a struct implementing [`Iterator`].
    pub fn iter(&self) -> Iter {
        Iter {
            inner: self.content.as_slice(),
            indices_start: self.indices_start,
            current_index: 0,
        }
    }

    /// Returns `true` if the witness contains no element.
    pub const fn is_empty(&self) -> bool {
        self.witness_elements == 0
    }

    /// Convenience method to create an array of byte-arrays from this witness.
    pub fn to_bytes(&self) -> Vec<Vec<u8>> {
        self.iter().map(|s| s.to_vec()).collect()
    }

    /// Convenience method to create an array of byte-arrays from this witness.
    pub fn to_vec(&self) -> Vec<Vec<u8>> {
        self.to_bytes()
    }

    /// Creates a [`Witness`] object from a slice of bytes slices where each slice is a witness item.
    pub fn from_slice<T: AsRef<[u8]>>(slice: &[T]) -> Self {
        let witness_elements = slice.len();
        let index_size = witness_elements * 4;
        let content_size = slice
            .iter()
            .map(|elem| elem.as_ref().len() + VarInt::from(elem.as_ref().len()).size())
            .sum();

        let mut content = vec![0u8; content_size + index_size];
        let mut cursor = 0usize;
        for (i, elem) in slice.iter().enumerate() {
            encode_cursor(&mut content, content_size, i, cursor);
            let elem_len_varint = VarInt::from(elem.as_ref().len());
            elem_len_varint
                .encode(&mut &mut content[cursor..cursor + elem_len_varint.size()])
                .expect("writers on vec don't errors, space granted by content_size");
            cursor += elem_len_varint.size();
            content[cursor..cursor + elem.as_ref().len()].copy_from_slice(elem.as_ref());
            cursor += elem.as_ref().len();
        }

        Self {
            witness_elements,
            content,
            indices_start: content_size,
        }
    }
}

impl Encodable for Witness {
    fn encode<W: Write + ?Sized>(&self, w: &mut W) -> Result<usize, std::io::Error> {
        let len = VarInt::from(self.witness_elements);
        len.encode(w)?;
        let content_with_indices_len = self.content.len();
        let indices_size = self.witness_elements * 4;
        let content_len = content_with_indices_len - indices_size;
        w.emit_slice(&self.content[..content_len])?;
        Ok(content_len + len.size())
    }
}

/// An iterator returning individual witness elements.
pub struct Iter<'a> {
    inner: &'a [u8],
    indices_start: usize,
    current_index: usize,
}

impl<'a> Iterator for Iter<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        let index = decode_cursor(self.inner, self.indices_start, self.current_index)?;
        let varint = VarInt::decode(&mut &self.inner[index..]).ok()?;
        let start = index + varint.size();
        let end = start + varint.0 as usize;
        let slice = &self.inner[start..end];
        self.current_index += 1;
        Some(slice)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let total_count = (self.inner.len() - self.indices_start) / 4;
        let remaining = total_count - self.current_index;
        (remaining, Some(remaining))
    }
}

impl<'a> ExactSizeIterator for Iter<'a> {}

impl<'a> IntoIterator for &'a Witness {
    type IntoIter = Iter<'a>;
    type Item = &'a [u8];

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl Decodable for Witness {
    fn decode<R: BufRead + ?Sized>(r: &mut R) -> Result<Self, std::io::Error> {
        let witness_elements = VarInt::decode(r)?.0 as usize;
        // Minimum size of witness element is 1 byte, so if the count is
        // greater than MAX_VEC_SIZE we must return an error.
        if witness_elements > MAX_VEC_SIZE {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "OversizedVectorAllocation",
            ));
        }
        if witness_elements == 0 {
            Ok(Self::default())
        } else {
            // Leave space at the head for element positions.
            // We will rotate them to the end of the Vec later.
            let witness_index_space = witness_elements * 4;
            let mut cursor = witness_index_space;

            // this number should be determined as high enough to cover most witness, and low enough
            // to avoid wasting space without reallocating
            let mut content = vec![0u8; cursor + 128];

            for i in 0..witness_elements {
                let element_size_varint = VarInt::decode(r)?;
                let element_size_varint_len = element_size_varint.size();
                let element_size = element_size_varint.0 as usize;
                let required_len = cursor
                    .checked_add(element_size)
                    .ok_or_else(|| {
                        std::io::Error::new(std::io::ErrorKind::Other, "OversizedVectorAllocation")
                    })?
                    .checked_add(element_size_varint_len)
                    .ok_or_else(|| {
                        std::io::Error::new(std::io::ErrorKind::Other, "OversizedVectorAllocation")
                    })?;
                if required_len > MAX_VEC_SIZE + witness_index_space {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "OversizedVectorAllocation",
                    ));
                }

                // We will do content.rotate_left(witness_index_space) later.
                // Encode the position's value AFTER we rotate left.
                encode_cursor(&mut content, 0, i, cursor - witness_index_space);

                resize_if_needed(&mut content, required_len);
                element_size_varint
                    .encode(&mut &mut content[cursor..cursor + element_size_varint_len])?;
                cursor += element_size_varint_len;
                r.read_exact(&mut content[cursor..cursor + element_size])?;
                cursor += element_size;
            }
            content.truncate(cursor);
            // Index space is now at the end of the Vec
            content.rotate_left(witness_index_space);
            Ok(Self {
                content,
                witness_elements,
                indices_start: cursor - witness_index_space,
            })
        }
    }
}

fn decode_cursor(bytes: &[u8], start_of_indices: usize, index: usize) -> Option<usize> {
    let start = start_of_indices + index * 4;
    let end = start + 4;
    if end > bytes.len() {
        None
    } else {
        Some(u32::from_ne_bytes(bytes[start..end].try_into().expect("is u32 size")) as usize)
    }
}
/// Correctness Requirements: value must always fit within u32
fn encode_cursor(bytes: &mut [u8], start_of_indices: usize, index: usize, value: usize) {
    let start = start_of_indices + index * 4;
    let end = start + 4;
    bytes[start..end].copy_from_slice(&u32::to_ne_bytes(
        value.try_into().expect("larger than u32"),
    ));
}

fn resize_if_needed(vec: &mut Vec<u8>, required_len: usize) {
    if required_len >= vec.len() {
        let mut new_len = vec.len().max(1);
        while new_len <= required_len {
            new_len *= 2;
        }
        vec.resize(new_len, 0);
    }
}

pub struct SerializeBytesAsHex<'a>(pub(crate) &'a [u8]);

impl<'a> serde::Serialize for SerializeBytesAsHex<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use hex::ToHex;

        serializer.collect_str(&format_args!("{}", self.0.encode_hex::<String>()))
    }
}

// Serde keep backward compatibility with old Vec<Vec<u8>> format
impl serde::Serialize for Witness {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeSeq;

        let human_readable = serializer.is_human_readable();
        let mut seq = serializer.serialize_seq(Some(self.witness_elements))?;

        // Note that the `Iter` strips the varints out when iterating.
        for elem in self.iter() {
            if human_readable {
                seq.serialize_element(&SerializeBytesAsHex(elem))?;
            } else {
                seq.serialize_element(&elem)?;
            }
        }
        seq.end()
    }
}

impl<'de> serde::Deserialize<'de> for Witness {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor; // Human-readable visitor.

        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = Witness;

            fn expecting(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                write!(f, "a sequence of hex arrays")
            }

            fn visit_seq<A: serde::de::SeqAccess<'de>>(
                self,
                mut a: A,
            ) -> Result<Self::Value, A::Error> {
                let mut ret = a.size_hint().map_or_else(Vec::new, Vec::with_capacity);

                while let Some(elem) = a.next_element::<String>()? {
                    let vec = hex::decode(&elem).map_err(|e| match e {
                        hex::FromHexError::InvalidHexCharacter { c, .. } => {
                            core::char::from_u32(c.into()).map_or_else(
                                || {
                                    serde::de::Error::invalid_value(
                                        serde::de::Unexpected::Other("invalid hex character"),
                                        &"a valid hex character",
                                    )
                                },
                                |c| {
                                    serde::de::Error::invalid_value(
                                        serde::de::Unexpected::Char(c),
                                        &"a valid hex character",
                                    )
                                },
                            )
                        }
                        hex::FromHexError::OddLength => {
                            serde::de::Error::invalid_length(0, &"an even length string")
                        }
                        hex::FromHexError::InvalidStringLength => {
                            serde::de::Error::invalid_length(0, &"an even length string")
                        }
                    })?;
                    ret.push(vec);
                }
                Ok(Witness::from_slice(&ret))
            }
        }

        if deserializer.is_human_readable() {
            deserializer.deserialize_seq(Visitor)
        } else {
            let vec: Vec<Vec<u8>> = serde::Deserialize::deserialize(deserializer)?;
            Ok(Self::from_slice(&vec))
        }
    }
}
