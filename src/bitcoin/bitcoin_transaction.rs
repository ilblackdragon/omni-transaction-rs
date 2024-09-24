use std::io::{BufRead, Write};

use borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::serde::{Deserialize, Serialize};

use super::{
    constants::{SEGWIT_FLAG, SEGWIT_MARKER},
    encoding::{decode::MAX_VEC_SIZE, utils::VarInt, Decodable, Encodable, ToU64},
    types::{EcdsaSighashType, LockTime, ScriptBuf, TransactionType, TxIn, TxOut, Version},
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
    pub fn build_for_signing(&self, sighash_type: EcdsaSighashType) -> Vec<u8> {
        let mut buffer = self.encode_fields();

        // Sighash type
        buffer.extend_from_slice(&(sighash_type as u32).to_le_bytes());

        buffer
    }

    pub fn build_with_script_sig(
        &mut self,
        input_index: usize,
        script_sig: ScriptBuf,
        tx_type: TransactionType,
    ) -> Vec<u8> {
        match tx_type {
            TransactionType::P2PKH | TransactionType::P2SH => {
                self.input[input_index].script_sig = script_sig;
            }
            _ => {
                panic!("Not implemented");
            }
        }

        self.encode_fields()
    }

    fn encode_fields(&self) -> Vec<u8> {
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
}

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
        let max_capacity = MAX_VEC_SIZE / 4 / std::mem::size_of::<Self>();
        let mut ret = Self::with_capacity(core::cmp::min(len as usize, max_capacity));
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
        let mut ret = Self::with_capacity(core::cmp::min(len as usize, max_capacity));
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
    use crate::bitcoin::types::{
        Amount as OmniAmount, EcdsaSighashType as OmniSighashType, Hash as OmniHash,
        OutPoint as OmniOutPoint, ScriptBuf as OmniScriptBuf, Sequence as OmniSequence,
        Txid as OmniTxid, Witness as OmniWitness,
    };

    // Rust Bitcoin imports
    use bitcoin::absolute::LockTime as RustBitcoinLockTime;
    use bitcoin::consensus::Encodable;
    use bitcoin::hashes::Hash;
    use bitcoin::sighash::{EcdsaSighashType, SighashCache};
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
        let mut tx = RustBitcoinTransaction {
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

        let sighash_type: EcdsaSighashType = EcdsaSighashType::All;
        let sighasher = SighashCache::new(&mut tx);
        let mut buffer: Vec<u8> = Vec::new();
        sighasher
            .legacy_encode_signing_data_to(
                &mut buffer,
                0,
                &ScriptBuf::default(),
                sighash_type.to_u32(),
            )
            .is_sighash_single_bug()
            .unwrap();

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

        let serialized = omni_tx.build_for_signing(OmniSighashType::All);

        assert_eq!(buffer.len(), serialized.len());
        assert_eq!(buffer, serialized);
    }

    #[test]
    fn test_build_for_signing_for_bitcoin_against_rust_bitcoin_for_version_2() {
        let height = 1000000;
        let version = 2;
        let mut tx = RustBitcoinTransaction {
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

        let sighash_type: EcdsaSighashType = EcdsaSighashType::All;
        let sighasher = SighashCache::new(&mut tx);
        let mut buffer: Vec<u8> = Vec::new();
        sighasher
            .legacy_encode_signing_data_to(
                &mut buffer,
                0,
                &ScriptBuf::default(),
                sighash_type.to_u32(),
            )
            .is_sighash_single_bug()
            .unwrap();

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

        let serialized = omni_tx.build_for_signing(OmniSighashType::All);
        println!("serialized BTC Omni: {:?}", serialized);

        assert_eq!(buffer.len(), serialized.len());
        assert_eq!(buffer, serialized);
    }
}
