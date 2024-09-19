use std::io::{BufRead, Write};

use borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::serde::{Deserialize, Serialize};

use super::{
    constants::{SEGWIT_FLAG, SEGWIT_MARKER},
    encoding::{decode::MAX_VEC_SIZE, utils::VarInt, Decodable, Encodable, ToU64},
    types::{LockTime, TxIn, TxOut, Version},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct BitcoinTransaction {
    /// The protocol version, is currently expected to be 1 or 2 (BIP 68).
    pub version: Version,
    /// Block height or timestamp. Transaction cannot be included in a block until this height/time.
    ///
    /// ### Relevant BIPs
    ///
    /// * [BIP-65 OP_CHECKLOCKTIMEVERIFY](https://github.com/bitcoin/bips/blob/master/bip-0065.mediawiki)
    /// * [BIP-113 Median time-past as endpoint for lock-time calculations](https://github.com/bitcoin/bips/blob/master/bip-0113.mediawiki)
    pub lock_time: LockTime,
    /// List of transaction inputs.
    pub input: Vec<TxIn>,
    /// List of transaction outputs.
    pub output: Vec<TxOut>,
}

impl BitcoinTransaction {
    pub fn build_for_signing(&self) -> Vec<u8> {
        let mut buffer = Vec::new();

        // Version
        self.version.encode(&mut buffer).unwrap();

        let uses_segwit_serialization =
            self.input.iter().any(|input| !input.witness.is_empty()) || self.input.is_empty();

        // BIP-141 (segwit) transaction serialization should include marker and flag.
        if uses_segwit_serialization {
            buffer.push(SEGWIT_MARKER);
            buffer.push(SEGWIT_FLAG);
        }

        self.input.encode(&mut buffer).unwrap();
        self.output.encode(&mut buffer).unwrap();

        // BIP-141 (segwit) transaction serialization also contains witness data.
        if uses_segwit_serialization {
            for input in &self.input {
                input.witness.encode(&mut buffer).unwrap();
            }
        }

        // Locktime
        self.lock_time.encode(&mut buffer).unwrap();

        buffer
    }

    // pub fn build_with_signature(&self, signature: Signature) -> Vec<u8> {
    //     let signed_tx = SignedTransaction {
    //         transaction: self.clone(),
    //         signature,
    //     };
    //     borsh::to_vec(&signed_tx).expect("failed to serialize Bitcoin transaction")
    // }

    // pub fn from_json(json: &str) -> Result<Self, near_sdk::serde_json::Error> {
    //     near_sdk::serde_json::from_str(json)
    // }
}

// build for signing
// build with signature

impl Encodable for Vec<TxIn> {
    fn encode<W: Write + ?Sized>(&self, w: &mut W) -> core::result::Result<usize, std::io::Error> {
        let mut len = 0;
        len += VarInt(self.len().to_u64()).encode(w)?;
        for c in self.iter() {
            len += c.encode(w)?;
        }
        Ok(len)
    }
}

impl Decodable for Vec<TxIn> {
    fn decode_from_finite_reader<R: BufRead + ?Sized>(
        r: &mut R,
    ) -> core::result::Result<Self, std::io::Error> {
        let len = VarInt::decode_from_finite_reader(r)?.0;
        // Do not allocate upfront more items than if the sequence of type
        // occupied roughly quarter a block. This should never be the case
        // for normal data, but even if that's not true - `push` will just
        // reallocate.
        // Note: OOM protection relies on reader eventually running out of
        // data to feed us.
        let max_capacity = MAX_VEC_SIZE / 4 / std::mem::size_of::<Vec<TxIn>>();
        let mut ret = Vec::with_capacity(core::cmp::min(len as usize, max_capacity));
        for _ in 0..len {
            ret.push(Decodable::decode_from_finite_reader(r)?);
        }
        Ok(ret)
    }
}

impl Encodable for Vec<TxOut> {
    fn encode<W: Write + ?Sized>(&self, w: &mut W) -> core::result::Result<usize, std::io::Error> {
        let mut len = 0;
        len += VarInt(self.len().to_u64()).encode(w)?;
        for c in self.iter() {
            len += c.encode(w)?;
        }
        Ok(len)
    }
}

impl Decodable for Vec<TxOut> {
    fn decode_from_finite_reader<R: BufRead + ?Sized>(
        r: &mut R,
    ) -> core::result::Result<Self, std::io::Error> {
        let len = VarInt::decode_from_finite_reader(r)?.0;
        // Do not allocate upfront more items than if the sequence of type
        // occupied roughly quarter a block. This should never be the case
        // for normal data, but even if that's not true - `push` will just
        // reallocate.
        // Note: OOM protection relies on reader eventually running out of
        // data to feed us.
        let max_capacity = MAX_VEC_SIZE / 4 / std::mem::size_of::<Vec<TxIn>>();
        let mut ret = Vec::with_capacity(core::cmp::min(len as usize, max_capacity));
        for _ in 0..len {
            ret.push(Decodable::decode_from_finite_reader(r)?);
        }
        Ok(ret)
    }
}
#[cfg(test)]
mod tests {
    // Omni imports
    use super::BitcoinTransaction as OmniBitcoinTransaction;
    use super::*;
    // use super::{LockTime as OmniLockTime, Version as OmniVersion};
    use crate::bitcoin::types::script_buf::ScriptBuf as OmniScriptBuf;
    use crate::bitcoin::types::tx_in::{
        hash::Hash as OmniHash, outpoint::OutPoint as OmniOutPoint,
        sequence::Sequence as OmniSequence, tx_id::Txid as OmniTxid,
        witness::Witness as OmniWitness,
    };
    use crate::bitcoin::types::tx_out::amount::Amount as OmniAmount;

    // Rust Bitcoin imports
    use bitcoin::absolute::LockTime as RustBitcoinLockTime;
    use bitcoin::consensus::Encodable;
    use bitcoin::hashes::Hash;
    use bitcoin::transaction::Sequence as RustBitcoinSequence;
    use bitcoin::transaction::{
        OutPoint, TxIn as RustBitcoinTxIn, TxOut as RustBitcoinTxOut, Txid,
    };
    use bitcoin::transaction::{
        Transaction as RustBitcoinTransaction, Version as RustBitcoinVersion,
    };
    use bitcoin::Witness;
    use bitcoin::{Amount, ScriptBuf};

    #[test]
    fn test_build_for_signing_for_bitcoin_against_rust_bitcoin_for_version_1() {
        let height = 1000000;
        let version = 1;
        let tx = RustBitcoinTransaction {
            version: RustBitcoinVersion(version),
            lock_time: RustBitcoinLockTime::from_height(height).unwrap(),
            input: vec![RustBitcoinTxIn {
                previous_output: OutPoint {
                    txid: Txid::from_raw_hash(Hash::all_zeros()),
                    vout: 0,
                },
                script_sig: ScriptBuf::default(),
                sequence: RustBitcoinSequence::default(),
                witness: Witness::default(),
            }],
            output: vec![RustBitcoinTxOut {
                value: Amount::from_sat(10000),
                script_pubkey: ScriptBuf::default(),
            }],
        };

        let mut buf = Vec::new();
        let size = tx.consensus_encode(&mut buf).unwrap();

        println!("size: {:?}", size);
        println!("serialized: {:?}", buf);

        // Omni implementation
        let omni_tx = OmniBitcoinTransaction {
            version: Version::One,
            lock_time: LockTime::from_height(height).unwrap(),
            input: vec![TxIn {
                previous_output: OmniOutPoint {
                    txid: OmniTxid(OmniHash::all_zeros()),
                    vout: 0,
                },
                script_sig: OmniScriptBuf::default(),
                sequence: OmniSequence::default(),
                witness: OmniWitness::default(),
            }],
            output: vec![TxOut {
                value: OmniAmount::from_sat(10000),
                script_pubkey: OmniScriptBuf::default(),
            }],
        };

        let serialized = omni_tx.build_for_signing();
        println!("serialized: {:?}", serialized);

        assert_eq!(size, serialized.len());
        assert_eq!(buf, serialized);
        // let mut buf = [0u8; 1024];
        // let size = omni_tx.build_for_signing(&mut &mut buf[..]).unwrap();
        // let mut buf = [0u8; 1024];
        // let raw_tx = hex!(SOME_TX);
        // let tx: Transaction = Decodable::consensus_decode(&mut raw_tx.as_slice()).unwrap();

        //
        // assert_eq!(size, SOME_TX.len() / 2);
        // assert_eq!(raw_tx, &buf[..size]);
        // let serialized = tx.build_for_signing();
        // println!("serialized: {:?}", serialized);

        // let tx = Transaction::from_bytes(&serialized).unwrap();
        // println!("tx: {:?}", tx);
    }

    #[test]
    fn test_build_for_signing_for_bitcoin_against_rust_bitcoin_for_version_2() {
        let height = 1000000;
        let version = 2;
        let tx = RustBitcoinTransaction {
            version: RustBitcoinVersion(version),
            lock_time: RustBitcoinLockTime::from_height(height).unwrap(),
            input: vec![RustBitcoinTxIn {
                previous_output: OutPoint {
                    txid: Txid::from_raw_hash(Hash::all_zeros()),
                    vout: 0,
                },
                script_sig: ScriptBuf::default(),
                sequence: RustBitcoinSequence::default(),
                witness: Witness::default(),
            }],
            output: vec![RustBitcoinTxOut {
                value: Amount::from_sat(10000),
                script_pubkey: ScriptBuf::default(),
            }],
        };

        let mut buf = Vec::new();
        let size = tx.consensus_encode(&mut buf).unwrap();

        println!("size: {:?}", size);
        println!("serialized BTC Orig: {:?}", buf);

        // Omni implementation
        let omni_tx = OmniBitcoinTransaction {
            version: Version::Two,
            lock_time: LockTime::from_height(height).unwrap(),
            input: vec![TxIn {
                previous_output: OmniOutPoint {
                    txid: OmniTxid(OmniHash::all_zeros()),
                    vout: 0,
                },
                script_sig: OmniScriptBuf::default(),
                sequence: OmniSequence::default(),
                witness: OmniWitness::default(),
            }],
            output: vec![TxOut {
                value: OmniAmount::from_sat(10000),
                script_pubkey: OmniScriptBuf::default(),
            }],
        };

        let serialized = omni_tx.build_for_signing();
        println!("serialized BTC Omni: {:?}", serialized);

        assert_eq!(size, serialized.len());
        assert_eq!(buf, serialized);
    }
}
