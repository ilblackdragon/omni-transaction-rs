use std::io::{BufRead, Write};

use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};

use crate::bitcoin::encoding::{Decodable, Encodable};

use super::tx_id::Txid;

/// A reference to a transaction output.
///
/// ### Bitcoin Core References
///
/// * [COutPoint definition](https://github.com/bitcoin/bitcoin/blob/345457b542b6a980ccfbc868af0970a6f91d1b82/src/primitives/transaction.h#L26)
#[derive(
    Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize, BorshSerialize, BorshDeserialize,
)]
pub struct OutPoint {
    /// The referenced transaction's txid.
    pub txid: Txid,
    /// The index of the referenced output in its transaction's vout.
    pub vout: u32,
}

impl OutPoint {
    /// The number of bytes that an outpoint contributes to the size of a transaction.
    pub const SIZE: usize = 32 + 4; // The serialized lengths of txid and vout.

    pub const fn new(txid: Txid, vout: u32) -> Self {
        Self { txid, vout }
    }

    /// Creates a "null" `OutPoint`.
    ///
    /// This value is used for coinbase transactions because they don't have any previous outputs.
    pub const fn null() -> Self {
        Self {
            txid: Txid::all_zeros(),
            vout: u32::MAX,
        }
    }

    /// Checks if an `OutPoint` is "null".
    pub fn is_null(&self) -> bool {
        *self == Self::null()
    }
}

impl Default for OutPoint {
    fn default() -> Self {
        Self::null()
    }
}

impl Encodable for OutPoint {
    fn encode<W: Write + ?Sized>(&self, w: &mut W) -> Result<usize, std::io::Error> {
        let mut len = 0;
        len += self.txid.encode(w)?;
        len += self.vout.encode(w)?;
        Ok(len)
    }
}

impl Decodable for OutPoint {
    fn decode<R: BufRead + ?Sized>(r: &mut R) -> Result<Self, std::io::Error> {
        let txid = Txid::decode(r)?;
        let vout = Decodable::decode(r)?;
        Ok(Self { txid, vout })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode() {
        let outpoint = OutPoint {
            txid: Txid::all_zeros(),
            vout: u32::MAX,
        };

        let mut buf = Vec::new();
        outpoint.encode(&mut buf).unwrap();
        assert_eq!(buf.len(), OutPoint::SIZE);

        let decoded_outpoint = OutPoint::decode_from_finite_reader(&mut buf.as_slice()).unwrap();
        assert_eq!(decoded_outpoint, outpoint);
    }
}
