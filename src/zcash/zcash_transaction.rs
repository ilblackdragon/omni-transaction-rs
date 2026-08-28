//! Zcash transaction
//!
//! Transparent-only v5 ([ZIP-225]) transaction with the [ZIP-244] txid and
//! signature digests. The signature digest returned by
//! [`ZcashTransaction::build_for_signing`] is the **final** 32-byte value that
//! is signed directly with ECDSA/secp256k1 (no extra hashing step), which
//! makes it directly usable as the payload of a NEAR MPC `sign` call on the
//! secp256k1 domain.
//!
//! [ZIP-225]: https://zips.z.cash/zip-0225
//! [ZIP-244]: https://zips.z.cash/zip-0244
#[cfg(feature = "borsh")]
use borsh::{BorshDeserialize, BorshSerialize};
#[cfg(feature = "schemars")]
use schemars::JsonSchema;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use super::constants::{V5_TX_VERSION, V5_VERSION_GROUP_ID};
use super::sighash;
use super::types::{ConsensusBranchId, ScriptBuf, SpentUtxo, TxIn, TxOut, ZcashSighashType};
use crate::bitcoin::encoding::Encodable;

/// Panic message for writes into a `Vec<u8>`, which cannot fail.
const VEC_WRITE: &str = "writing to a Vec<u8> cannot fail";

/// A transparent-only Zcash v5 ([ZIP-225]/NU5+) transaction.
///
/// The `header` (`0x80000005`) and `nVersionGroupId` (`0x26A7270A`) fields are
/// not part of the struct: this module only builds v5 transactions, so they
/// are emitted from constants. The Sapling and Orchard bundles are always
/// empty (serialized as the three bytes `00 00 00`).
///
/// Only non-coinbase spends are supported: coinbase transactions use a
/// different [ZIP-244] `transparent_sig_digest` rule that this module does not
/// implement.
///
/// ###### Example:
///
/// You can create a Zcash transaction directly in your NEAR contract or Rust
/// client, or from a JSON string.
///
/// ```rust
/// use omni_transaction::zcash::types::{
///     Amount, ConsensusBranchId, Hash, OutPoint, ScriptBuf, Sequence, SpentUtxo, TxIn, TxOut,
///     Txid, Witness, ZcashSighashType,
/// };
///
/// let omni_tx = omni_transaction::zcash::ZcashTransaction {
///     consensus_branch_id: ConsensusBranchId::Nu6_3,
///     lock_time: 0,
///     expiry_height: 2_500_000,
///     input: vec![TxIn {
///         previous_output: OutPoint {
///             txid: Txid(Hash::all_zeros()),
///             vout: 0,
///         },
///         script_sig: ScriptBuf::default(), // For P2PKH the script_sig is initially empty.
///         sequence: Sequence::ENABLE_LOCKTIME_NO_RBF,
///         witness: Witness::default(), // Unused by Zcash; must stay empty.
///     }],
///     output: vec![TxOut {
///         value: Amount::from_sat(99_990_000),
///         script_pubkey: ScriptBuf::from_hex("76a9148132712c3ff19f3a151234616777420a6d7ef22688ac")
///             .unwrap(),
///     }],
/// };
///
/// // The ZIP-244 sighash commits to the value and scriptPubKey of every spent coin.
/// let spent_utxos = vec![SpentUtxo::new(
///     100_000_000,
///     ScriptBuf::from_hex("76a914507173527b4c3318a2aecd793bf1cfed705950cf88ac").unwrap(),
/// )];
///
/// // The final 32-byte sighash, signed directly with ECDSA/secp256k1.
/// let sighash = omni_tx.build_for_signing(0, ZcashSighashType::All, &spent_utxos);
/// assert_eq!(sighash.len(), 32);
///
/// // If you prefer to build the transaction from a JSON:
/// # #[cfg(feature = "serde_json")]
/// # let _ = {
/// let json_value = r#"
/// {
///     "consensus_branch_id": "Nu6_3",
///     "lock_time": 0,
///     "expiry_height": 2500000,
///     "input": [{
///         "previous_output": {
///             "txid": "1f1e1d1c1b1a191817161514131211100f0e0d0c0b0a09080706050403020100",
///             "vout": 1
///         },
///         "script_sig": [],
///         "sequence": 4294967294,
///         "witness": []
///     }],
///     "output": [{
///         "value": 99990000,
///         "script_pubkey": "76a9148132712c3ff19f3a151234616777420a6d7ef22688ac"
///     }]
/// }
/// "#;
/// let tx = omni_transaction::zcash::ZcashTransaction::from_json(json_value).unwrap();
/// # };
/// ```
///
/// [ZIP-225]: https://zips.z.cash/zip-0225
/// [ZIP-244]: https://zips.z.cash/zip-0244
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "borsh", derive(BorshSerialize, BorshDeserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct ZcashTransaction {
    /// The consensus branch id of the network upgrade epoch the transaction
    /// will be mined in (`nConsensusBranchId`).
    ///
    /// Consensus-checked: a wrong value makes the transaction invalid, so
    /// callers should re-check the active branch id at broadcast time around
    /// network upgrades.
    pub consensus_branch_id: ConsensusBranchId,
    /// Block height or timestamp before which the transaction cannot be mined
    /// (`lock_time`), exactly as in Bitcoin.
    pub lock_time: u32,
    /// Block height after which the transaction expires and is evicted from
    /// mempools (`nExpiryHeight`, [ZIP-203]).
    ///
    /// `0` means the transaction never expires; otherwise the value must be in
    /// `1..=499_999_999`. `zcashd` uses tip height + 40 by default.
    ///
    /// [ZIP-203]: https://zips.z.cash/zip-0203
    pub expiry_height: u32,
    /// List of transparent inputs.
    ///
    /// The reused Bitcoin [`TxIn`] carries a `witness` field that has no
    /// equivalent in the Zcash v5 format; it must stay empty and is never
    /// serialized.
    pub input: Vec<TxIn>,
    /// List of transparent outputs.
    pub output: Vec<TxOut>,
}

impl ZcashTransaction {
    /// Encodes the full v5 transaction into a vector of bytes ([ZIP-225]).
    ///
    /// The layout is `header || nVersionGroupId || nConsensusBranchId ||
    /// lock_time || nExpiryHeight || vin || vout || 00 00 || 00`, where the
    /// trailing three bytes are the empty Sapling and Orchard bundles. The
    /// result is the canonical wire format: hex-encode it for
    /// `sendrawtransaction`.
    ///
    /// [ZIP-225]: https://zips.z.cash/zip-0225
    pub fn serialize(&self) -> Vec<u8> {
        let mut buffer = Vec::new();
        buffer.extend_from_slice(&V5_TX_VERSION.to_le_bytes());
        buffer.extend_from_slice(&V5_VERSION_GROUP_ID.to_le_bytes());
        buffer.extend_from_slice(&self.consensus_branch_id.to_u32().to_le_bytes());
        buffer.extend_from_slice(&self.lock_time.to_le_bytes());
        buffer.extend_from_slice(&self.expiry_height.to_le_bytes());
        self.input.encode(&mut buffer).expect(VEC_WRITE);
        self.output.encode(&mut buffer).expect(VEC_WRITE);
        // Empty Sapling bundle: nSpendsSapling = 0, nOutputsSapling = 0.
        // All other Sapling fields are present iff those counts are non-zero.
        buffer.extend_from_slice(&[0x00, 0x00]);
        // Empty Orchard bundle: nActionsOrchard = 0. All other Orchard fields
        // are present iff the count is non-zero.
        buffer.push(0x00);
        buffer
    }

    /// Computes the final 32-byte [ZIP-244] signature digest (sighash) for the
    /// transparent input at `input_index`.
    ///
    /// Unlike the Bitcoin module (which returns a preimage the caller must
    /// double-SHA256), this is the exact value Zcash consensus verifies the
    /// ECDSA/secp256k1 signature against — do **not** hash it again. Sign it
    /// directly (e.g. pass it as the payload of a NEAR MPC `sign` call on the
    /// secp256k1 domain), then low-S normalize, DER-encode, and append the
    /// `hash_type` byte before building the script sig.
    ///
    /// The digest commits to the value and `scriptPubKey` of **every** coin
    /// spent by the transaction, so `spent_utxos` must contain one
    /// [`SpentUtxo`] per input, in input order. Script sig contents never
    /// enter the digest, so every input can be signed before any script sig is
    /// attached.
    ///
    /// # Panics
    ///
    /// * If `input_index` is out of bounds.
    /// * If `spent_utxos.len() != self.input.len()`.
    ///
    /// [ZIP-244]: https://zips.z.cash/zip-0244
    pub fn build_for_signing(
        &self,
        input_index: usize,
        sighash_type: ZcashSighashType,
        spent_utxos: &[SpentUtxo],
    ) -> Vec<u8> {
        sighash::signature_digest_with_bundle_digests(
            self,
            input_index,
            sighash_type,
            spent_utxos,
            &sighash::empty_sapling_digest(),
            &sighash::empty_orchard_digest(),
        )
        .to_vec()
    }

    /// Attaches a script sig to the input at `input_index` and returns the
    /// full serialized v5 transaction, ready for `sendrawtransaction`.
    ///
    /// For P2PKH inputs build the script sig with
    /// [`p2pkh_script_sig`](crate::zcash::utils::p2pkh_script_sig) from the
    /// DER-encoded low-S signature with the appended `hash_type` byte and the
    /// SEC1-encoded public key. For P2SH, construct the pushes and the redeem
    /// script yourself, as in the Bitcoin module.
    ///
    /// # Panics
    ///
    /// If `input_index` is out of bounds.
    pub fn build_with_signature(&mut self, input_index: usize, script_sig: ScriptBuf) -> Vec<u8> {
        assert!(
            input_index < self.input.len(),
            "input_index out of bounds: {} inputs, index {}",
            self.input.len(),
            input_index
        );
        self.input[input_index].script_sig = script_sig;
        // Called via UFCS: with the `serde` feature on, `self.serialize()` on
        // a `&mut self` receiver would resolve to serde's blanket
        // `impl Serialize for &mut T` instead of the inherent method.
        Self::serialize(self)
    }

    /// Alias of [`Self::build_with_signature`], named for consistency with the
    /// Bitcoin module's `build_with_script_sig`.
    ///
    /// # Panics
    ///
    /// If `input_index` is out of bounds.
    pub fn build_with_script_sig(&mut self, input_index: usize, script_sig: ScriptBuf) -> Vec<u8> {
        self.build_with_signature(input_index, script_sig)
    }

    /// Computes the [ZIP-244] txid of the transaction, in *internal* byte
    /// order (explorers and RPC display the byte-reversed hex).
    ///
    /// Script sigs are not part of the v5 txid, so it is stable across
    /// signing (non-malleable).
    ///
    /// [ZIP-244]: https://zips.z.cash/zip-0244
    pub fn txid(&self) -> [u8; 32] {
        sighash::txid_digest_with_bundle_digests(
            self,
            &sighash::empty_sapling_digest(),
            &sighash::empty_orchard_digest(),
        )
    }

    /// Deserializes a JSON representation of the transaction into a
    /// [`ZcashTransaction`].
    ///
    /// The `previous_output.txid` field uses the same convention as the
    /// Bitcoin module: display-order hex (byte-reversed relative to the wire
    /// encoding).
    ///
    /// # Errors
    ///
    /// Returns a [`serde_json::Error`] when the JSON is malformed or does not
    /// match the expected shape.
    #[cfg(feature = "serde_json")]
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        let tx: Self = serde_json::from_str(json)?;
        Ok(tx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::zcash::types::{
        Amount, Hash, OutPoint, ScriptBuf, Sequence, SpentUtxo, TxIn, TxOut, Txid, Witness,
    };

    /// Unsigned serialization of the transparent-only end-to-end vector
    /// (independently generated by an implementation that reproduces all 10
    /// official zcash-test-vectors zip_0244 vectors byte-for-byte).
    const E2E_UNSIGNED_HEX: &str = "050000800a27a7265b16a53700000000a025260001000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f0100000000feffffff02f0b9f505000000001976a9148132712c3ff19f3a151234616777420a6d7ef22688ac881300000000000017a914ad63d2abbb1395c6a8bbdac575cca6e6e63a1e1c87000000";
    /// SIGHASH_ALL signature digest of input 0 of the end-to-end vector.
    const E2E_SIGHASH_ALL_HEX: &str =
        "e057d65831080c539cb2eb9ae99a5b33799b38964e56c968a9922afb8a93c883";
    /// ZIP-244 header digest (T.1/S.1) of the end-to-end vector.
    const E2E_HEADER_DIGEST_HEX: &str =
        "0abb52d7f63869e075096d924e90cd2840b84f2372aa336641eb53abcb6fb6d4";
    /// ZIP-244 txid (internal byte order) of the end-to-end vector.
    const E2E_TXID_HEX: &str = "982ac085f6fd0dc814bc1ef302551fd530585704fe698b820a3ceb026171b8e0";
    /// P2PKH script sig with a fixed 72-byte DER signature + hash type and a
    /// 33-byte compressed public key.
    const E2E_SCRIPT_SIG_HEX: &str = "4830450221111111111111111111111111111111111111111111111111111111111111111111022022222222222222222222222222222222222222222222222222222222222222220121023333333333333333333333333333333333333333333333333333333333333333";
    /// Signed serialization of the end-to-end vector with the fixed script sig
    /// attached to input 0.
    const E2E_SIGNED_HEX: &str = "050000800a27a7265b16a53700000000a025260001000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f010000006b4830450221111111111111111111111111111111111111111111111111111111111111111111022022222222222222222222222222222222222222222222222222222222222222220121023333333333333333333333333333333333333333333333333333333333333333feffffff02f0b9f505000000001976a9148132712c3ff19f3a151234616777420a6d7ef22688ac881300000000000017a914ad63d2abbb1395c6a8bbdac575cca6e6e63a1e1c87000000";

    /// Builds a `TxIn` whose previous txid is given in *internal* byte order
    /// (as serialized on the wire). `Hash` stores display-order bytes and
    /// reverses them when encoding, so the bytes are reversed here.
    fn txin_from_internal_txid(txid_internal_hex: &str, vout: u32, sequence: u32) -> TxIn {
        let mut bytes: [u8; 32] = hex::decode(txid_internal_hex).unwrap().try_into().unwrap();
        bytes.reverse();
        TxIn {
            previous_output: OutPoint::new(Txid(Hash(bytes)), vout),
            script_sig: ScriptBuf::default(),
            sequence: Sequence(sequence),
            witness: Witness::default(),
        }
    }

    fn end_to_end_tx() -> ZcashTransaction {
        ZcashTransaction {
            consensus_branch_id: crate::zcash::types::ConsensusBranchId::Nu6_3,
            lock_time: 0,
            expiry_height: 2_500_000,
            input: vec![txin_from_internal_txid(
                "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f",
                1,
                0xFFFF_FFFE,
            )],
            output: vec![
                TxOut {
                    value: Amount::from_sat(99_990_000),
                    script_pubkey: ScriptBuf::from_hex(
                        "76a9148132712c3ff19f3a151234616777420a6d7ef22688ac",
                    )
                    .unwrap(),
                },
                TxOut {
                    value: Amount::from_sat(5_000),
                    script_pubkey: ScriptBuf::from_hex(
                        "a914ad63d2abbb1395c6a8bbdac575cca6e6e63a1e1c87",
                    )
                    .unwrap(),
                },
            ],
        }
    }

    fn end_to_end_spent_utxos() -> Vec<SpentUtxo> {
        vec![SpentUtxo::new(
            100_000_000,
            ScriptBuf::from_hex("76a914507173527b4c3318a2aecd793bf1cfed705950cf88ac").unwrap(),
        )]
    }

    #[test]
    fn test_end_to_end_unsigned_serialization() {
        let tx = end_to_end_tx();
        let serialized = tx.serialize();
        assert_eq!(serialized.len(), 132);
        assert_eq!(hex::encode(serialized), E2E_UNSIGNED_HEX);
    }

    #[test]
    fn test_end_to_end_header_digest() {
        let tx = end_to_end_tx();
        assert_eq!(
            hex::encode(crate::zcash::sighash::header_digest(&tx)),
            E2E_HEADER_DIGEST_HEX
        );
    }

    #[test]
    fn test_end_to_end_build_for_signing_sighash_all() {
        let tx = end_to_end_tx();
        let sighash = tx.build_for_signing(0, ZcashSighashType::All, &end_to_end_spent_utxos());
        assert_eq!(hex::encode(sighash), E2E_SIGHASH_ALL_HEX);
    }

    #[test]
    fn test_end_to_end_txid() {
        let tx = end_to_end_tx();
        assert_eq!(hex::encode(tx.txid()), E2E_TXID_HEX);
    }

    #[test]
    fn test_end_to_end_build_with_signature() {
        let mut tx = end_to_end_tx();
        let script_sig = ScriptBuf::from_hex(E2E_SCRIPT_SIG_HEX).unwrap();
        let signed = tx.build_with_signature(0, script_sig);
        assert_eq!(hex::encode(signed), E2E_SIGNED_HEX);
    }

    #[test]
    fn test_end_to_end_build_with_script_sig_alias() {
        let mut tx = end_to_end_tx();
        let script_sig = ScriptBuf::from_hex(E2E_SCRIPT_SIG_HEX).unwrap();
        let signed = tx.build_with_script_sig(0, script_sig);
        assert_eq!(hex::encode(signed), E2E_SIGNED_HEX);
    }

    #[test]
    fn test_txid_is_stable_across_signing() {
        let mut tx = end_to_end_tx();
        let txid_before = tx.txid();
        tx.build_with_signature(0, ScriptBuf::from_hex(E2E_SCRIPT_SIG_HEX).unwrap());
        assert_eq!(tx.txid(), txid_before);
    }

    #[test]
    #[should_panic(expected = "input_index out of bounds")]
    fn test_build_with_signature_panics_on_out_of_bounds_index() {
        let mut tx = end_to_end_tx();
        tx.build_with_signature(1, ScriptBuf::default());
    }

    #[test]
    #[cfg(all(feature = "serde", feature = "serde_json"))]
    fn test_from_json_zcash_transaction() {
        // Txid in display order (byte-reversed internal order), matching the
        // Bitcoin module's JSON convention.
        let json = r#"
        {
            "consensus_branch_id": "Nu6_3",
            "lock_time": 0,
            "expiry_height": 2500000,
            "input": [{
                "previous_output": {
                    "txid": "1f1e1d1c1b1a191817161514131211100f0e0d0c0b0a09080706050403020100",
                    "vout": 1
                },
                "script_sig": [],
                "sequence": 4294967294,
                "witness": []
            }],
            "output": [
                {
                    "value": 99990000,
                    "script_pubkey": "76a9148132712c3ff19f3a151234616777420a6d7ef22688ac"
                },
                {
                    "value": 5000,
                    "script_pubkey": "a914ad63d2abbb1395c6a8bbdac575cca6e6e63a1e1c87"
                }
            ]
        }
        "#;

        let tx = ZcashTransaction::from_json(json).unwrap();
        assert_eq!(tx, end_to_end_tx());
        assert_eq!(hex::encode(tx.serialize()), E2E_UNSIGNED_HEX);
    }

    #[test]
    #[cfg(all(feature = "serde", feature = "serde_json"))]
    fn test_from_json_custom_branch_id() {
        let json = r#"
        {
            "consensus_branch_id": { "Custom": 911772449 },
            "lock_time": 0,
            "expiry_height": 0,
            "input": [],
            "output": []
        }
        "#;

        let tx = ZcashTransaction::from_json(json).unwrap();
        assert_eq!(tx.consensus_branch_id.to_u32(), 911_772_449);
    }
}
