use std::io::{BufRead, Write};

use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};

use crate::bitcoin::encoding::{Decodable, Encodable};

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

impl Decodable for Sequence {
    fn decode<R: BufRead + ?Sized>(r: &mut R) -> Result<Self, std::io::Error> {
        Decodable::decode(r).map(Sequence)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode() {
        let sequence = Sequence(42);
        let mut buf = Vec::new();

        assert_eq!(sequence.encode(&mut buf).unwrap(), 4);
        assert_eq!(Sequence::decode(&mut buf.as_slice()).unwrap(), sequence);
    }
}
