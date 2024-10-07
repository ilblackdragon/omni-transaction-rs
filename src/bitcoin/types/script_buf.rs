use core::fmt;
use std::io::{BufRead, Write};

use borsh::{BorshDeserialize, BorshSerialize};
use schemars::JsonSchema;

use crate::bitcoin::encoding::{encode::Encodable, Decodable};

#[derive(Debug, Default, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize, JsonSchema)]
pub struct ScriptBuf(pub Vec<u8>);

impl ScriptBuf {
    /// Creates a [`ScriptBuf`] from a hex string.
    pub fn from_hex(s: &str) -> Result<Self, String> {
        let v = Vec::from_hex(s)?;
        Ok(Self::from_bytes(v))
    }

    /// Converts byte vector into script.
    ///
    /// This method doesn't (re)allocate.
    pub const fn from_bytes(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }
}

pub trait FromHex: Sized {
    /// Error type returned while parsing hex string.
    type Error: Sized + fmt::Debug + fmt::Display;

    /// Produces an object from a hex string.
    fn from_hex(s: &str) -> Result<Self, Self::Error>;
}

impl FromHex for Vec<u8> {
    type Error = String;

    fn from_hex(s: &str) -> Result<Self, Self::Error> {
        hex::decode(s).map_err(|e| e.to_string())
    }
}

impl Encodable for ScriptBuf {
    fn encode<W: Write + ?Sized>(&self, w: &mut W) -> Result<usize, std::io::Error> {
        self.0.encode(w)
    }
}

impl Decodable for ScriptBuf {
    fn decode_from_finite_reader<R: BufRead + ?Sized>(r: &mut R) -> Result<Self, std::io::Error> {
        Ok(Self(Decodable::decode_from_finite_reader(r)?))
    }
}

impl serde::Serialize for ScriptBuf {
    /// User-facing serialization for `Script`.
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_bytes(&self.0)
    }
}

impl<'de> serde::Deserialize<'de> for ScriptBuf {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use core::fmt::Formatter;

        if deserializer.is_human_readable() {
            struct Visitor;
            impl<'de> serde::de::Visitor<'de> for Visitor {
                type Value = ScriptBuf;

                fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
                    println!("expecting");
                    formatter.write_str("a script hex")
                }

                // fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
                // where
                //     E: serde::de::Error,
                // {
                //     Ok(ScriptBuf::from_hex(v).unwrap())
                // }

                fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
                where
                    E: serde::de::Error,
                {
                    if v.is_empty() {
                        Ok(ScriptBuf(vec![]))
                    } else {
                        ScriptBuf::from_hex(v).map_err(E::custom)
                    }
                }

                fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
                where
                    A: serde::de::SeqAccess<'de>,
                {
                    let mut vec = Vec::new();
                    while let Some(byte) = seq.next_element()? {
                        vec.push(byte);
                    }
                    Ok(ScriptBuf(vec))
                }
            }
            // deserializer.deserialize_str(Visitor)
            deserializer.deserialize_any(Visitor)
        } else {
            struct BytesVisitor;

            impl<'de> serde::de::Visitor<'de> for BytesVisitor {
                type Value = ScriptBuf;

                fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
                    formatter.write_str("a script Vec<u8>")
                }

                fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
                where
                    E: serde::de::Error,
                {
                    Ok(ScriptBuf::from_bytes(v.to_vec()))
                }

                fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Self::Value, E>
                where
                    E: serde::de::Error,
                {
                    Ok(ScriptBuf::from_bytes(v))
                }
            }
            deserializer.deserialize_byte_buf(BytesVisitor)
        }
    }
}
