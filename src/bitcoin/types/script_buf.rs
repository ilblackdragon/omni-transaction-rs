use core::fmt;
use std::io::{BufRead, Write};

use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};

use crate::bitcoin::encoding::{encode::Encodable, Decodable};

#[derive(
    Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize, BorshSerialize, BorshDeserialize,
)]
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
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
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
