use std::io::{BufRead, Write};

use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};

use crate::bitcoin::{
    encoding::{Decodable, Encodable},
    types::script_buf::ScriptBuf,
};

use super::amount::Amount;

/// Bitcoin transaction output.
///
/// Defines new coins to be created as a result of the transaction,
/// along with spending conditions ("script", aka "output script"),
/// which an input spending it must satisfy.
///
/// An output that is not yet spent by an input is called Unspent Transaction Output ("UTXO").
///
/// ### Bitcoin Core References
///
/// * [CTxOut definition](https://github.com/bitcoin/bitcoin/blob/345457b542b6a980ccfbc868af0970a6f91d1b82/src/primitives/transaction.h#L148)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct TxOut {
    /// The value of the output, in satoshis.
    pub value: Amount,
    /// The script which must be satisfied for the output to be spent.
    pub script_pubkey: ScriptBuf,
}

impl Encodable for TxOut {
    fn encode<W: Write + ?Sized>(&self, w: &mut W) -> Result<usize, std::io::Error> {
        let mut len = 0;
        len += self.value.encode(w)?;
        len += self.script_pubkey.encode(w)?;
        Ok(len)
    }
}

impl Decodable for TxOut {
    fn decode_from_finite_reader<R: BufRead + ?Sized>(r: &mut R) -> Result<Self, std::io::Error> {
        let value = Decodable::decode_from_finite_reader(r)?;
        let script_pubkey = Decodable::decode_from_finite_reader(r)?;
        Ok(Self {
            value,
            script_pubkey,
        })
    }
}
