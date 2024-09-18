use std::io::Write;

use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};

use crate::bitcoin::encoding::Encodable;

/// Bitcoin transaction input sequence number.
#[derive(
    Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize, BorshSerialize, BorshDeserialize,
)]
pub struct Sequence(pub u32);

impl Sequence {
    /// The number of bytes that a sequence number contributes to the size of a transaction.
    pub const SIZE: usize = 4; // Serialized length of a u32.
}

impl Encodable for Sequence {
    fn encode<W: Write + ?Sized>(&self, w: &mut W) -> Result<usize, std::io::Error> {
        self.0.encode(w)
    }
}

// impl Decodable for Sequence {
//     fn consensus_decode<R: BufRead + ?Sized>(r: &mut R) -> Result<Self, encode::Error> {
//         Decodable::consensus_decode(r).map(Sequence)
//     }
// }
