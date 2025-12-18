use std::io::{BufRead, Write};

use borsh::{BorshDeserialize, BorshSerialize};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::bitcoin::encoding::{Decodable, Encodable};

/// Bitcoin transaction input sequence number.
#[derive(
    Debug,
    Copy,
    Clone,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    BorshSerialize,
    BorshDeserialize,
    JsonSchema,
)]
pub struct Sequence(pub u32);

impl Sequence {
    /// The number of bytes that a sequence number contributes to the size of a transaction.
    pub const SIZE: usize = 4; // Serialized length of a u32.

    /// The maximum allowable sequence number.
    ///
    /// This sequence number disables absolute lock time and replace-by-fee.
    pub const MAX: Self = Self(0xFFFFFFFF);
    /// Zero value sequence.
    ///
    /// This sequence number enables replace-by-fee and absolute lock time.
    pub const ZERO: Self = Self(0);

    /// The sequence number that enables absolute lock time but disables replace-by-fee
    /// and relative lock time.
    pub const ENABLE_LOCKTIME_NO_RBF: Self = Self::MIN_NO_RBF;
    /// The sequence number that enables replace-by-fee and absolute lock time but
    /// disables relative lock time.
    pub const ENABLE_RBF_NO_LOCKTIME: Self = Self(0xFFFFFFFD);

    /// The lowest sequence number that does not opt-in for replace-by-fee.
    ///
    /// A transaction is considered to have opted in to replacement of itself
    /// if any of it's inputs have a `Sequence` number less than this value
    /// (Explicit Signalling [BIP-125]).
    ///
    /// [BIP-125]: <https://github.com/bitcoin/bips/blob/master/bip-0125.mediawiki]>
    const MIN_NO_RBF: Self = Self(0xFFFFFFFE);
}

impl Default for Sequence {
    /// The default value of sequence is 0xffffffff.
    fn default() -> Self {
        Self::MAX
    }
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
