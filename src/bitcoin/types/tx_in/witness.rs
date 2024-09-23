use std::io::{BufRead, Write};

use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};

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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
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
        Witness {
            content: Vec::new(),
            witness_elements: 0,
            indices_start: 0,
        }
    }

    /// Returns `true` if the witness contains no element.
    pub fn is_empty(&self) -> bool {
        self.witness_elements == 0
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
            Ok(Witness::default())
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
                    .ok_or(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "OversizedVectorAllocation",
                    ))?
                    .checked_add(element_size_varint_len)
                    .ok_or(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "OversizedVectorAllocation",
                    ))?;
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
            Ok(Witness {
                content,
                witness_elements,
                indices_start: cursor - witness_index_space,
            })
        }
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
