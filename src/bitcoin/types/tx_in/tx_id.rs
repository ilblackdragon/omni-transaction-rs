use std::io::Write;

use crate::bitcoin::encoding::Encodable;

use super::hash::Hash;
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};

#[derive(
    Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize, BorshSerialize, BorshDeserialize,
)]
pub struct Txid(Hash);

impl Txid {
    pub fn as_byte_array(&self) -> [u8; 32] {
        self.0.as_byte_array()
    }
}

impl Txid {
    /// The "all zeros" TXID.
    ///
    /// This is used as the "txid" of the dummy input of a coinbase transaction. It is
    /// not a real TXID and should not be used in other contexts.
    pub fn all_zeros() -> Self {
        Txid(Hash::all_zeros())
    }
}

impl Encodable for Txid {
    fn encode<W: Write + ?Sized>(&self, w: &mut W) -> Result<usize, std::io::Error> {
        self.0.encode(w)
    }
}
