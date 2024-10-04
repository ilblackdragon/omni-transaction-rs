use borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::{BufRead, Write};

use super::{
    constants::{SEGWIT_FLAG, SEGWIT_MARKER},
    encoding::{decode::MAX_VEC_SIZE, utils::VarInt, Decodable, Encodable, ToU64},
    types::{
        EcdsaSighashType, LockTime, ScriptBuf, TransactionType, TxIn, TxOut, Version, Witness,
    },
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

// Function to compute sha256d (double SHA-256)
fn sha256d(data: &[u8]) -> Vec<u8> {
    let hash1 = Sha256::digest(data);
    let hash2 = Sha256::digest(hash1);
    hash2.to_vec()
}

impl BitcoinTransaction {
    // Common
    pub fn serialize(&self) -> Vec<u8> {
        let mut buffer = Vec::new();

        let _ = self.encode(&mut buffer);

        buffer
    }

    // Legacy
    pub fn build_for_signing_legacy(&self, sighash_type: EcdsaSighashType) -> Vec<u8> {
        let mut buffer = Vec::new();

        let _ = self.encode(&mut buffer);

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
            TransactionType::P2WPKH | TransactionType::P2WSH => {
                panic!("Use build_with_witness for SegWit transactions");
            }
        }

        let mut buffer = Vec::new();
        let _ = self.encode(&mut buffer);

        buffer
    }

    // Segwit
    pub fn build_for_signing_segwit(
        &self,
        sighash_type: EcdsaSighashType,
        input_index: usize,
        script_code: &ScriptBuf,
        value: u64,
    ) -> Vec<u8> {
        if self.version != Version::Two {
            panic!("SegWit transactions must be version 2");
        }

        let mut buffer = Vec::new();

        self.encode_for_sighash_for_segwit(&mut buffer, input_index, script_code, value);

        // Sighash type
        buffer.extend_from_slice(&(sighash_type as u32).to_le_bytes());

        buffer
    }

    pub fn build_with_witness(
        &mut self,
        input_index: usize,
        witness: Vec<Vec<u8>>,
        tx_type: TransactionType,
    ) -> Vec<u8> {
        match tx_type {
            TransactionType::P2WPKH | TransactionType::P2WSH => {
                self.input[input_index].witness = Witness::from_slice(&witness);
            }
            TransactionType::P2PKH | TransactionType::P2SH => {
                panic!("Use build_with_script_sig for non-SegWit transactions");
            }
        }

        let mut buffer = Vec::new();

        let _ = self
            .encode(&mut buffer)
            .expect("Failed to encode transaction");

        buffer
    }

    fn encode_for_sighash_for_segwit(
        &self,
        buffer: &mut Vec<u8>,
        input_index: usize,
        script_code: &ScriptBuf,
        value: u64,
    ) {
        // Version
        self.version.encode(buffer).unwrap();

        let has_witness = self.input.iter().any(|input| !input.witness.is_empty());

        if has_witness {
            // Marker and Flag
            buffer.push(SEGWIT_MARKER);
            buffer.push(SEGWIT_FLAG);
        }

        // Hash prevouts
        let mut prevouts = Vec::new();
        for input in &self.input {
            input.previous_output.encode(&mut prevouts).unwrap();
        }
        let prevouts_hash = sha256d(&prevouts);
        buffer.extend_from_slice(&prevouts_hash);

        // Hash sequences
        let mut sequences = Vec::new();
        for input in &self.input {
            input.sequence.encode(&mut sequences).unwrap();
        }
        let sequences_hash = sha256d(&sequences);
        buffer.extend_from_slice(&sequences_hash);

        // Outpoint
        self.input[input_index]
            .previous_output
            .encode(buffer)
            .unwrap();

        // Script code
        script_code.encode(buffer).unwrap();

        // Value
        buffer.extend_from_slice(&value.to_le_bytes());

        // Sequence
        self.input[input_index].sequence.encode(buffer).unwrap();

        // Hash outputs
        let mut outputs = Vec::new();
        for output in &self.output {
            output.encode(&mut outputs).unwrap();
        }
        let outputs_hash = sha256d(&outputs);
        buffer.extend_from_slice(&outputs_hash);

        // Locktime
        self.lock_time.encode(buffer).unwrap();
    }

    /// Returns whether or not to serialize transaction as specified in BIP-144.
    fn uses_segwit_serialization(&self) -> bool {
        if self.input.iter().any(|input| !input.witness.is_empty()) {
            return true;
        }
        // To avoid serialization ambiguity, no inputs means we use BIP141 serialization
        self.input.is_empty()
    }

    pub fn from_json(json: &str) -> Result<Self, near_sdk::serde_json::Error> {
        let tx: Self = near_sdk::serde_json::from_str(json)?;
        Ok(tx)
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

impl Encodable for BitcoinTransaction {
    fn encode<W: Write + ?Sized>(&self, w: &mut W) -> Result<usize, std::io::Error> {
        let mut len = 0;
        len += self.version.encode(w)?;

        // Legacy transaction serialization format only includes inputs and outputs.
        if !self.uses_segwit_serialization() {
            len += self.input.encode(w)?;
            len += self.output.encode(w)?;
        } else {
            // BIP-141 (segwit) transaction serialization also includes marker, flag, and witness data.
            len += SEGWIT_MARKER.encode(w)?;
            len += SEGWIT_FLAG.encode(w)?;
            len += self.input.encode(w)?;
            len += self.output.encode(w)?;
            for input in &self.input {
                len += input.witness.encode(w)?;
            }
        }
        len += self.lock_time.encode(w)?;
        Ok(len)
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
    fn test_build_for_signing_against_rust_bitcoin_for_version_1() {
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

        let serialized = omni_tx.build_for_signing_legacy(OmniSighashType::All);

        assert_eq!(buffer.len(), serialized.len());
        assert_eq!(buffer, serialized);
    }

    #[test]
    fn test_build_for_signing_for_against_rust_bitcoin_for_version_2() {
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

        let serialized = omni_tx.build_for_signing_legacy(OmniSighashType::All);
        println!("serialized BTC Omni: {:?}", serialized);

        assert_eq!(buffer.len(), serialized.len());
        assert_eq!(buffer, serialized);
    }

    #[test]
    fn test_build_for_signing_against_rust_bitcoin_for_version_2_and_segwit() {
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
        let mut sighasher = SighashCache::new(&mut tx);
        let mut buffer: Vec<u8> = Vec::new();
        sighasher
            .segwit_v0_encode_signing_data_to(
                &mut buffer,
                0,
                &ScriptBuf::default(),
                Amount::from_sat(0),
                sighash_type,
            )
            .unwrap(); // Handle the Result

        println!("serialized buffer {:?}", buffer);
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

        let serialized = omni_tx.build_for_signing_segwit(
            OmniSighashType::All,
            0,
            &OmniScriptBuf::default(),
            OmniAmount::from_sat(0).to_sat(),
        );
        println!("serialized BTC Omni: {:?}", serialized);

        assert_eq!(buffer.len(), serialized.len());
        assert_eq!(buffer, serialized);
    }

    #[test]
    fn test_from_json_bitcoin_transaction() {
        let json = r#"
        {
            "version": "1",
            "lock_time": "0",
            "input": [
                {
                    "previous_output": {
                        "txid": "bc25cc0dddd0a202c21e66521a692c0586330a9a9dcc38ccd9b4d2093037f31a",
                        "vout": 0
                    },
                    "script_sig": "",
                    "sequence": 4294967295,
                    "witness": []
                }
            ],
            "output": [
                {
                    "value": 1,
                    "script_pubkey": "76a9148356ecd5f1761e60c144dc2f4de6bf7d8be7690688ad"
                },
                {
                    "value": 2649,
                    "script_pubkey": "76a9148356ecd5f1761e60c144dc2f4de6bf7d8be7690688ac"
                }
            ]
        }
        "#;

        let tx = OmniBitcoinTransaction::from_json(json).unwrap();
        println!("tx: {:?}", tx);
    }

    #[test]
    fn test_from_json_bitcoin_transaction_2() {
        let json = r#"
        {
            "version": "1",
            "lock_time": "0",
            "input": [
                {
                    "previous_output": {
                        "txid": "bc25cc0dddd0a202c21e66521a692c0586330a9a9dcc38ccd9b4d2093037f31a",
                        "vout": 0
                    },
                    "script_sig": [],
                    "sequence": 4294967295,
                    "witness": []
                }
            ],
            "output": [
                {
                    "value": 1,
                    "script_pubkey": "76a9148356ecd5f1761e60c144dc2f4de6bf7d8be7690688ad"
                },
                {
                    "value": 2649,
                    "script_pubkey": "76a9148356ecd5f1761e60c144dc2f4de6bf7d8be7690688ac"
                }
            ]
        }
        "#;

        let tx = OmniBitcoinTransaction::from_json(json).unwrap();
        println!("tx: {:?}", tx);
    }
}
