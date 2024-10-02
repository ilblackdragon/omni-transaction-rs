use std::io::{self, BufRead, Write};

use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};

use crate::bitcoin::encoding::{Decodable, Encodable};
use crate::bitcoin::types::script_buf::ScriptBuf;

use super::{outpoint::OutPoint, sequence::Sequence, witness::Witness};

/// Bitcoin transaction input.
///
/// It contains the location of the previous transaction's output,
/// that it spends and set of scripts that satisfy its spending
/// conditions.
///
/// ### Bitcoin Core References
///
/// * [CTxIn definition](https://github.com/bitcoin/bitcoin/blob/345457b542b6a980ccfbc868af0970a6f91d1b82/src/primitives/transaction.h#L65)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct TxIn {
    /// The reference to the previous output that is being used as an input.
    pub previous_output: OutPoint,
    /// The script which pushes values on the stack which will cause
    /// the referenced output's script to be accepted.
    pub script_sig: ScriptBuf,
    /// The sequence number, which suggests to miners which of two
    /// conflicting transactions should be preferred, or 0xFFFFFFFF
    /// to ignore this feature. This is generally never used since
    /// the miner behavior cannot be enforced.
    pub sequence: Sequence,
    /// Witness data: an array of byte-arrays.
    /// Note that this field is *not* (de)serialized with the rest of the TxIn in
    /// Encodable/Decodable, as it is (de)serialized at the end of the full
    /// Transaction. It *is* (de)serialized with the rest of the TxIn in other
    /// (de)serialization routines.
    pub witness: Witness,
}

impl Encodable for TxIn {
    fn encode<W: Write + ?Sized>(&self, w: &mut W) -> Result<usize, io::Error> {
        let mut len = 0;
        let previous_output_len = self.previous_output.encode(w)?;
        len += previous_output_len;

        let script_sig_len = self.script_sig.encode(w)?;
        len += script_sig_len;

        let sequence_len = self.sequence.encode(w)?;
        len += sequence_len;

        Ok(len)
    }
}

impl Decodable for TxIn {
    fn decode<R: BufRead + ?Sized>(r: &mut R) -> Result<Self, std::io::Error> {
        let previous_output = OutPoint::decode(r)?;
        let script_sig = ScriptBuf::decode(r)?;
        let sequence = Sequence::decode(r)?;
        Ok(Self {
            previous_output,
            script_sig,
            sequence,
            witness: Witness::default(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode() {
        let txin = TxIn {
            previous_output: OutPoint::null(),
            script_sig: ScriptBuf::default(),
            sequence: Sequence(0),
            witness: Witness::default(),
        };
        let mut buf = Vec::new();

        // The expected size is 36 (OutPoint = 32 txid + 4 vout) + 1 (ScriptBuf) + + 4 (Sequence) = 41
        assert_eq!(txin.encode(&mut buf).unwrap(), 41);
        assert_eq!(TxIn::decode(&mut buf.as_slice()).unwrap(), txin);
    }
}
