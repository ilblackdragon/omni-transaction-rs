use std::io::BufRead;

use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};

use crate::bitcoin::encoding::{encode::Encodable, extensions::WriteExt, Decodable};

#[derive(
    Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize, BorshSerialize, BorshDeserialize,
)]
pub struct Hash([u8; 32]);

impl Hash {
    pub fn as_byte_array(&self) -> [u8; 32] {
        self.0
    }
}

impl Hash {
    pub fn all_zeros() -> Self {
        Hash([0; 32])
    }
}

impl Encodable for Hash {
    fn encode<W: WriteExt + ?Sized>(&self, w: &mut W) -> Result<usize, std::io::Error> {
        w.emit_slice(&self.0).map(|_| self.0.len())
    }
}

impl Decodable for Hash {
    fn decode<R: BufRead + ?Sized>(r: &mut R) -> Result<Self, std::io::Error> {
        let mut buf = [0; 32];
        r.read_exact(&mut buf)?;
        Ok(Hash(buf))
    }
}
