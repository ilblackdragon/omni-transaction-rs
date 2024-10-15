use core::fmt;
use std::{io::BufRead, str::FromStr};

use borsh::{BorshDeserialize, BorshSerialize};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::bitcoin::encoding::{encode::Encodable, extensions::WriteExt, Decodable};

#[derive(
    Debug,
    Copy,
    Clone,
    Eq,
    PartialEq,
    Serialize,
    Deserialize,
    BorshSerialize,
    BorshDeserialize,
    JsonSchema,
)]
#[serde(crate = "near_sdk::serde")]
pub struct Hash(pub [u8; 32]);

impl Hash {
    pub const fn as_byte_array(&self) -> [u8; 32] {
        self.0
    }

    pub fn from_hex(hex: &str) -> Result<Self, hex::FromHexError> {
        let bytes = hex::decode(hex)?;
        Ok(Self(bytes.try_into().expect("Invalid length")))
    }
}

impl Hash {
    pub const fn all_zeros() -> Self {
        Self([0; 32])
    }
}

impl Encodable for Hash {
    fn encode<W: WriteExt + ?Sized>(&self, w: &mut W) -> Result<usize, std::io::Error> {
        w.emit_slice(&self.0.iter().rev().cloned().collect::<Vec<u8>>())
            .map(|_| self.0.len())
    }
}

impl Decodable for Hash {
    fn decode<R: BufRead + ?Sized>(r: &mut R) -> Result<Self, std::io::Error> {
        let mut buf: [u8; 32] = [0; 32];
        r.read_exact(&mut buf)?; // Read 32 bytes from the buffer
        Ok(Self(
            buf.iter()
                .rev() // Reverse the bytes to convert from little-endian to big-endian
                .cloned()
                .collect::<Vec<u8>>()
                .try_into()
                .unwrap(), // Convert the bytes into a Hash instance
        ))
    }
}

impl FromStr for Hash {
    type Err = hex::FromHexError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_hex(s)
    }
}

use hex::encode;

impl fmt::Display for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", encode(self.0))
    }
}
