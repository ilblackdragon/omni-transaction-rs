//! [ZIP-244] txid and signature digest tree for v5 transactions.
//!
//! Every node of the tree is a BLAKE2b-256 hash with a 16-byte ASCII
//! personalization. The digest of an empty input list/bundle is the
//! personalized hash of the empty byte array.
//!
//! The bundle digests (Sapling T.3/S.3 and Orchard T.4/S.4) are parameters of
//! the internal functions so the transparent logic can be unit-tested against
//! the official `zcash-test-vectors` `zip_0244` vectors (which all carry
//! shielded bundles); the public API always passes the empty-bundle digests.
//!
//! [ZIP-244]: https://zips.z.cash/zip-0244
use blake2b_simd::Params;

use super::constants::{V5_TX_VERSION, V5_VERSION_GROUP_ID};
use super::types::{SpentUtxo, TxOut, ZcashSighashType};
use super::zcash_transaction::ZcashTransaction;
use crate::bitcoin::encoding::Encodable;

/// Personalization prefix of the top-level txid/signature digest; the 4-byte
/// little-endian consensus branch id is appended to form the full 16 bytes.
const PERSON_TX_HASH_PREFIX: &[u8; 12] = b"ZcashTxHash_";
/// Personalization of the header digest (T.1 / S.1).
const PERSON_HEADERS: &[u8; 16] = b"ZTxIdHeadersHash";
/// Personalization of both T.2 and S.2 (intentionally shared, ZIP-244 rationale).
const PERSON_TRANSPARENT: &[u8; 16] = b"ZTxIdTranspaHash";
/// Personalization of the prevouts digest (T.2a / S.2b).
const PERSON_PREVOUTS: &[u8; 16] = b"ZTxIdPrevoutHash";
/// Personalization of the sequence digest (T.2b / S.2e).
const PERSON_SEQUENCES: &[u8; 16] = b"ZTxIdSequencHash";
/// Personalization of the outputs digest (T.2c / S.2f).
const PERSON_OUTPUTS: &[u8; 16] = b"ZTxIdOutputsHash";
/// Personalization of the spent amounts digest (S.2c).
const PERSON_AMOUNTS: &[u8; 16] = b"ZTxTrAmountsHash";
/// Personalization of the spent scriptPubKeys digest (S.2d).
const PERSON_SCRIPTS: &[u8; 16] = b"ZTxTrScriptsHash";
/// Personalization of the per-input digest (S.2g); note the three underscores.
const PERSON_TXIN: &[u8; 16] = b"Zcash___TxInHash";
/// Personalization of the Sapling bundle digest (T.3 / S.3).
const PERSON_SAPLING: &[u8; 16] = b"ZTxIdSaplingHash";
/// Personalization of the Orchard bundle digest (T.4 / S.4).
const PERSON_ORCHARD: &[u8; 16] = b"ZTxIdOrchardHash";

/// Panic message for writes into a `Vec<u8>`, which cannot fail.
const VEC_WRITE: &str = "writing to a Vec<u8> cannot fail";

/// BLAKE2b-256 with a 16-byte personalization.
fn blake2b_256(personalization: &[u8; 16], data: &[u8]) -> [u8; 32] {
    let hash = Params::new()
        .hash_length(32)
        .personal(personalization)
        .hash(data);
    let mut out = [0u8; 32];
    out.copy_from_slice(hash.as_bytes());
    out
}

/// The 16-byte personalization of the top-level txid/signature digest:
/// `"ZcashTxHash_" || branch_id (little-endian)`.
fn tx_hash_personalization(consensus_branch_id: u32) -> [u8; 16] {
    let mut person = [0u8; 16];
    person[..12].copy_from_slice(PERSON_TX_HASH_PREFIX);
    person[12..].copy_from_slice(&consensus_branch_id.to_le_bytes());
    person
}

/// T.3/S.3: digest of an empty Sapling bundle.
pub fn empty_sapling_digest() -> [u8; 32] {
    blake2b_256(PERSON_SAPLING, &[])
}

/// T.4/S.4: digest of an empty Orchard bundle.
pub fn empty_orchard_digest() -> [u8; 32] {
    blake2b_256(PERSON_ORCHARD, &[])
}

/// T.1/S.1: header digest over the five little-endian u32 header fields.
pub fn header_digest(tx: &ZcashTransaction) -> [u8; 32] {
    let mut data = Vec::with_capacity(20);
    data.extend_from_slice(&V5_TX_VERSION.to_le_bytes());
    data.extend_from_slice(&V5_VERSION_GROUP_ID.to_le_bytes());
    data.extend_from_slice(&tx.consensus_branch_id.to_u32().to_le_bytes());
    data.extend_from_slice(&tx.lock_time.to_le_bytes());
    data.extend_from_slice(&tx.expiry_height.to_le_bytes());
    blake2b_256(PERSON_HEADERS, &data)
}

/// T.2a: digest of the concatenation of all 36-byte outpoints.
fn prevouts_digest(tx: &ZcashTransaction) -> [u8; 32] {
    let mut data = Vec::with_capacity(tx.input.len() * 36);
    for input in &tx.input {
        input.previous_output.encode(&mut data).expect(VEC_WRITE);
    }
    blake2b_256(PERSON_PREVOUTS, &data)
}

/// T.2b: digest of the concatenation of all little-endian u32 `nSequence` values.
fn sequence_digest(tx: &ZcashTransaction) -> [u8; 32] {
    let mut data = Vec::with_capacity(tx.input.len() * 4);
    for input in &tx.input {
        input.sequence.encode(&mut data).expect(VEC_WRITE);
    }
    blake2b_256(PERSON_SEQUENCES, &data)
}

/// T.2c: digest of the concatenation of the given serialized outputs
/// (`value u64 LE || CompactSize || scriptPubKey`).
fn outputs_digest(outputs: &[TxOut]) -> [u8; 32] {
    let mut data = Vec::new();
    for output in outputs {
        output.encode(&mut data).expect(VEC_WRITE);
    }
    blake2b_256(PERSON_OUTPUTS, &data)
}

/// S.2c: digest of the concatenation of the spent coins' values as
/// **signed** 64-bit little-endian integers.
fn amounts_digest(spent_utxos: &[SpentUtxo]) -> [u8; 32] {
    let mut data = Vec::with_capacity(spent_utxos.len() * 8);
    for utxo in spent_utxos {
        data.extend_from_slice(&utxo.value.to_le_bytes());
    }
    blake2b_256(PERSON_AMOUNTS, &data)
}

/// S.2d: digest of the concatenation of the spent coins' scriptPubKeys,
/// each with its leading CompactSize.
fn scriptpubkeys_digest(spent_utxos: &[SpentUtxo]) -> [u8; 32] {
    let mut data = Vec::new();
    for utxo in spent_utxos {
        utxo.script_pubkey.encode(&mut data).expect(VEC_WRITE);
    }
    blake2b_256(PERSON_SCRIPTS, &data)
}

/// S.2g: per-input digest committing to the outpoint, value and scriptPubKey
/// of the coin spent by the signed input, and the input's `nSequence`.
fn txin_sig_digest(tx: &ZcashTransaction, input_index: usize, spent_utxo: &SpentUtxo) -> [u8; 32] {
    let input = &tx.input[input_index];
    let mut data = Vec::with_capacity(36 + 8 + 1 + spent_utxo.script_pubkey.0.len() + 4);
    input.previous_output.encode(&mut data).expect(VEC_WRITE);
    data.extend_from_slice(&spent_utxo.value.to_le_bytes());
    spent_utxo.script_pubkey.encode(&mut data).expect(VEC_WRITE);
    input.sequence.encode(&mut data).expect(VEC_WRITE);
    blake2b_256(PERSON_TXIN, &data)
}

/// T.2: transparent digest of the transaction.
///
/// The personalized hash of the empty byte array when there are neither
/// transparent inputs nor outputs, otherwise the personalized hash of
/// `prevouts_digest || sequence_digest || outputs_digest`.
fn transparent_digest(tx: &ZcashTransaction) -> [u8; 32] {
    if tx.input.is_empty() && tx.output.is_empty() {
        blake2b_256(PERSON_TRANSPARENT, &[])
    } else {
        let mut data = Vec::with_capacity(96);
        data.extend_from_slice(&prevouts_digest(tx));
        data.extend_from_slice(&sequence_digest(tx));
        data.extend_from_slice(&outputs_digest(&tx.output));
        blake2b_256(PERSON_TRANSPARENT, &data)
    }
}

/// [ZIP-244] txid digest with caller-supplied Sapling/Orchard bundle digests.
///
/// The result is the txid in *internal* byte order (explorers display it
/// byte-reversed). Bundle digests are parameters for testability against the
/// official vectors; use [`empty_sapling_digest`]/[`empty_orchard_digest`]
/// for transparent-only transactions.
///
/// [ZIP-244]: https://zips.z.cash/zip-0244
pub fn txid_digest_with_bundle_digests(
    tx: &ZcashTransaction,
    sapling_digest: &[u8; 32],
    orchard_digest: &[u8; 32],
) -> [u8; 32] {
    let mut data = Vec::with_capacity(128);
    data.extend_from_slice(&header_digest(tx));
    data.extend_from_slice(&transparent_digest(tx));
    data.extend_from_slice(sapling_digest);
    data.extend_from_slice(orchard_digest);
    blake2b_256(
        &tx_hash_personalization(tx.consensus_branch_id.to_u32()),
        &data,
    )
}

/// [ZIP-244] signature digest for a transparent input, with caller-supplied
/// Sapling/Orchard bundle digests.
///
/// Returns the final 32-byte sighash that is signed directly with
/// ECDSA/secp256k1 (no extra hashing step). Only non-coinbase spends are
/// supported.
///
/// # Panics
///
/// * If `input_index` is out of bounds.
/// * If `spent_utxos.len() != tx.input.len()` (the digest commits to every
///   spent coin, see S.2c/S.2d).
///
/// [ZIP-244]: https://zips.z.cash/zip-0244
pub fn signature_digest_with_bundle_digests(
    tx: &ZcashTransaction,
    input_index: usize,
    sighash_type: ZcashSighashType,
    spent_utxos: &[SpentUtxo],
    sapling_digest: &[u8; 32],
    orchard_digest: &[u8; 32],
) -> [u8; 32] {
    assert!(
        input_index < tx.input.len(),
        "input_index out of bounds: {} inputs, index {}",
        tx.input.len(),
        input_index
    );
    assert_eq!(
        spent_utxos.len(),
        tx.input.len(),
        "one SpentUtxo per input is required (ZIP-244 commits to every spent coin)"
    );
    const BASE_NONE: u8 = 0x02;
    const BASE_SINGLE: u8 = 0x03;

    // S.2b..S.2e
    let (prevouts_sig, amounts_sig, scriptpubkeys_sig, sequence_sig) =
        if sighash_type.is_anyone_can_pay() {
            (
                blake2b_256(PERSON_PREVOUTS, &[]),
                blake2b_256(PERSON_AMOUNTS, &[]),
                blake2b_256(PERSON_SCRIPTS, &[]),
                blake2b_256(PERSON_SEQUENCES, &[]),
            )
        } else {
            (
                prevouts_digest(tx),
                amounts_digest(spent_utxos),
                scriptpubkeys_digest(spent_utxos),
                sequence_digest(tx),
            )
        };

    // S.2f
    let outputs_sig = match sighash_type.base_type() {
        BASE_SINGLE if input_index < tx.output.len() => {
            outputs_digest(&tx.output[input_index..=input_index])
        }
        // `SIGHASH_NONE`, and `SIGHASH_SINGLE` with no output at `input_index`,
        // both commit to the empty output list. ZIP-244 defines the latter as
        // the personalized empty hash rather than rejecting it (see the final
        // `else` of `outputs_sig_digest` in `zcash-test-vectors/zip_0244.py`),
        // so the digest is well defined; whether such a signature is useful is
        // the caller's decision.
        BASE_NONE | BASE_SINGLE => outputs_digest(&[]),
        _ => outputs_digest(&tx.output), // SIGHASH_ALL
    };

    // S.2g
    let txin_sig = txin_sig_digest(tx, input_index, &spent_utxos[input_index]);

    // S.2
    let mut transparent_data = Vec::with_capacity(1 + 32 * 6);
    transparent_data.push(sighash_type.to_u8());
    transparent_data.extend_from_slice(&prevouts_sig);
    transparent_data.extend_from_slice(&amounts_sig);
    transparent_data.extend_from_slice(&scriptpubkeys_sig);
    transparent_data.extend_from_slice(&sequence_sig);
    transparent_data.extend_from_slice(&outputs_sig);
    transparent_data.extend_from_slice(&txin_sig);
    let transparent_sig_digest = blake2b_256(PERSON_TRANSPARENT, &transparent_data);

    // S.1 || S.2 || S.3 || S.4
    let mut data = Vec::with_capacity(128);
    data.extend_from_slice(&header_digest(tx));
    data.extend_from_slice(&transparent_sig_digest);
    data.extend_from_slice(sapling_digest);
    data.extend_from_slice(orchard_digest);
    blake2b_256(
        &tx_hash_personalization(tx.consensus_branch_id.to_u32()),
        &data,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::zcash::types::{
        Amount, ConsensusBranchId, Hash, OutPoint, ScriptBuf, Sequence, SpentUtxo, TxIn, TxOut,
        Txid, Witness, ZcashSighashType,
    };

    /// Builds a `TxIn` from a ZIP-244 test-vector outpoint given as 36 hex-encoded
    /// bytes in *internal* order (32-byte txid || u32 LE vout) plus a sequence.
    fn txin_from_internal_outpoint(outpoint_hex: &str, sequence: u32) -> TxIn {
        let bytes = hex::decode(outpoint_hex).unwrap();
        assert_eq!(bytes.len(), 36);
        // `Hash` stores display-order bytes and reverses them when encoding,
        // so internal-order txid bytes must be reversed here.
        let mut txid_display: [u8; 32] = bytes[..32].try_into().unwrap();
        txid_display.reverse();
        let vout = u32::from_le_bytes(bytes[32..].try_into().unwrap());
        TxIn {
            previous_output: OutPoint::new(Txid(Hash(txid_display)), vout),
            script_sig: ScriptBuf::default(),
            sequence: Sequence(sequence),
            witness: Witness::default(),
        }
    }

    fn txout(value: u64, script_pubkey_hex: &str) -> TxOut {
        TxOut {
            value: Amount::from_sat(value),
            script_pubkey: ScriptBuf::from_hex(script_pubkey_hex).unwrap(),
        }
    }

    fn spent(value: i64, script_pubkey_hex: &str) -> SpentUtxo {
        SpentUtxo::new(value, ScriptBuf::from_hex(script_pubkey_hex).unwrap())
    }

    /// Official zcash-test-vectors zip_0244 vector 4: 2 transparent inputs,
    /// 2 transparent outputs, empty Sapling bundle, 1 Orchard action.
    fn official_vector_4() -> (ZcashTransaction, Vec<SpentUtxo>, [u8; 32]) {
        let tx = ZcashTransaction {
            consensus_branch_id: ConsensusBranchId::Nu5,
            lock_time: 2_377_642_351,
            expiry_height: 353_784_540,
            input: vec![
                txin_from_internal_outpoint(
                    "51d6006becf8d2ffb03990f67774a81e05b7f4bbad8577fa27c9de64e1b11dcf384f5956",
                    0x65C3_100B,
                ),
                txin_from_internal_outpoint(
                    "7ebac03bfc0b587bef2f45ec8acdaa51c143b0cb25b9142c61bd790a80d7c23f90cc0349",
                    0x3E84_D2E4,
                ),
            ],
            output: vec![
                txout(646_237_202_206_608, "636aacac6a005100"),
                txout(389_003_185_263_136, "ac6a6a00"),
            ],
        };
        let spent_utxos = vec![
            spent(754_044_915_413_924, "ac6a53656aac"),
            spent(637_640_651_332_574, "536aac6a00006a63"),
        ];
        let orchard_digest: [u8; 32] =
            hex::decode("bc5294d5d6904899268adc3da0edad8fa5b7e85314298928a3abe72f3a1accda")
                .unwrap()
                .try_into()
                .unwrap();
        (tx, spent_utxos, orchard_digest)
    }

    /// Official zcash-test-vectors zip_0244 vector 6: 2 transparent inputs,
    /// zero transparent outputs, empty Sapling bundle, 4 Orchard actions.
    fn official_vector_6() -> (ZcashTransaction, Vec<SpentUtxo>, [u8; 32]) {
        let tx = ZcashTransaction {
            consensus_branch_id: ConsensusBranchId::Nu5,
            lock_time: 1_080_739_450,
            expiry_height: 180_198_152,
            input: vec![
                txin_from_internal_outpoint(
                    "488eb7cf33f6dad1666a05f91ad7757965c29936e7fa48d77e89ee0962f58c051d11d055",
                    0x261B_8A08,
                ),
                txin_from_internal_outpoint(
                    "48b8174cbcfc8b5b5cd077115afde18405054e5da9a04310342c5d3b526e0b02c5ca1722",
                    0xD123_EEDE,
                ),
            ],
            output: vec![],
        };
        let spent_utxos = vec![
            spent(741_599_467_359_839, "5365ac"),
            spent(1_790_607_082_653_742, "005252ac6363525163"),
        ];
        let orchard_digest: [u8; 32] =
            hex::decode("238671e398cde31c414c1747dda8b3f2c2608cf55557df3f15cc22306de663e9")
                .unwrap()
                .try_into()
                .unwrap();
        (tx, spent_utxos, orchard_digest)
    }

    #[test]
    fn test_empty_bundle_digests() {
        assert_eq!(
            hex::encode(empty_sapling_digest()),
            "6f2fc8f98feafd94e74a0df4bed74391ee0b5a69945e4ced8ca8a095206f00ae"
        );
        assert_eq!(
            hex::encode(empty_orchard_digest()),
            "9fbe4ed13b0c08e671c11a3407d84e1117cd45028a2eee1b9feae78b48a6e2c1"
        );
    }

    #[test]
    fn test_official_vector_4_txid() {
        let (tx, _, orchard_digest) = official_vector_4();
        let txid = txid_digest_with_bundle_digests(&tx, &empty_sapling_digest(), &orchard_digest);
        assert_eq!(
            hex::encode(txid),
            "35ff79dca2b2492acf3ed9757a00a57892c661d2b68f229a6177c0f86feb2e4c"
        );
    }

    #[test]
    fn test_official_vector_4_sighash_all_input_0() {
        let (tx, spent_utxos, orchard_digest) = official_vector_4();
        let sighash = signature_digest_with_bundle_digests(
            &tx,
            0,
            ZcashSighashType::All,
            &spent_utxos,
            &empty_sapling_digest(),
            &orchard_digest,
        );
        assert_eq!(
            hex::encode(sighash),
            "92dc54223e4fd679b98c146f10d3a56fd81ab5dc843cb110af9857649eb518d5"
        );
    }

    /// Every non-ALL sighash type of official vector 4, input 0 (values from
    /// zcash-test-vectors zip_0244.json, columns sighash_none..single_anyone).
    #[test]
    fn test_official_vector_4_all_other_sighash_types_input_0() {
        let (tx, spent_utxos, orchard_digest) = official_vector_4();
        let cases = [
            (
                ZcashSighashType::None,
                "32d83ae0492ab432a582d612b9ccf8fa6fce9f80f8e543ee623a6c9d54e6bbeb",
            ),
            (
                ZcashSighashType::Single,
                "02244e35834dc43f03c8e5693bfa040a0bb3cb0de3b321ae97e2726b44c81133",
            ),
            (
                ZcashSighashType::AllPlusAnyoneCanPay,
                "38a33ae6a00237ff60209e8178a9ddb4a2ba61b75de42ca6efae1dfc3d8372f0",
            ),
            (
                ZcashSighashType::NonePlusAnyoneCanPay,
                "8c8c2dd50efa49d001a02a0481ee28fb201ca74fcdfc9dc849e8fe4177b86ae5",
            ),
            (
                ZcashSighashType::SinglePlusAnyoneCanPay,
                "f064a1783f6c0b89b0f4002fc1163562a0d7cb86510571074af864817d795776",
            ),
        ];
        for (sighash_type, expected) in cases {
            let sighash = signature_digest_with_bundle_digests(
                &tx,
                0,
                sighash_type,
                &spent_utxos,
                &empty_sapling_digest(),
                &orchard_digest,
            );
            assert_eq!(hex::encode(sighash), expected, "type {sighash_type:?}");
        }
    }

    /// Non-ALL sighash types of official vector 6, input 1 (no transparent
    /// outputs, so the SINGLE variants are undefined and covered by the
    /// panic test below).
    #[test]
    fn test_official_vector_6_other_sighash_types_input_1() {
        let (tx, spent_utxos, orchard_digest) = official_vector_6();
        let cases = [
            (
                ZcashSighashType::None,
                "00119abf9661cb873f0831c5377b8f31995b3f8f58c2a9682c617c3487acfd71",
            ),
            (
                ZcashSighashType::AllPlusAnyoneCanPay,
                "804a502ba04df2fbe077a5f666b9821abe250e4d362495d1a1239914a2ad6e07",
            ),
            (
                ZcashSighashType::NonePlusAnyoneCanPay,
                "0bbe5dc2bdb902696fb4c9306c145875505bd4b451cc9eb2c0ac985a2fa7a2e6",
            ),
        ];
        for (sighash_type, expected) in cases {
            let sighash = signature_digest_with_bundle_digests(
                &tx,
                1,
                sighash_type,
                &spent_utxos,
                &empty_sapling_digest(),
                &orchard_digest,
            );
            assert_eq!(hex::encode(sighash), expected, "type {sighash_type:?}");
        }
    }

    #[test]
    fn test_official_vector_6_txid() {
        let (tx, _, orchard_digest) = official_vector_6();
        let txid = txid_digest_with_bundle_digests(&tx, &empty_sapling_digest(), &orchard_digest);
        assert_eq!(
            hex::encode(txid),
            "1b66bbce2146b688a21ecd2baa0ba5c0c17c55344c00a22c57470bfd1499bc01"
        );
    }

    #[test]
    fn test_official_vector_6_sighash_all_input_1_with_no_outputs() {
        let (tx, spent_utxos, orchard_digest) = official_vector_6();
        let sighash = signature_digest_with_bundle_digests(
            &tx,
            1,
            ZcashSighashType::All,
            &spent_utxos,
            &empty_sapling_digest(),
            &orchard_digest,
        );
        assert_eq!(
            hex::encode(sighash),
            "941cbae22ac25e72df6a92ea3949137c9fb5a7b8c44a29f26c75860f3523b6a8"
        );
    }

    #[test]
    #[should_panic(expected = "one SpentUtxo per input is required")]
    fn test_sighash_panics_on_spent_utxo_count_mismatch() {
        let (tx, mut spent_utxos, orchard_digest) = official_vector_4();
        spent_utxos.pop();
        signature_digest_with_bundle_digests(
            &tx,
            0,
            ZcashSighashType::All,
            &spent_utxos,
            &empty_sapling_digest(),
            &orchard_digest,
        );
    }

    /// ZIP-244 S.2f: `SIGHASH_SINGLE` with no output at the signed input's
    /// index commits to the empty output list instead of being rejected.
    ///
    /// Vector 6 has zero transparent outputs, and the official
    /// `zip_0244.json` leaves its `sighash_single` columns null, so these two
    /// expected values were derived from an independent reimplementation of
    /// the S.2 digest tree which reproduces vector 6's four published
    /// `sighash_all`/`sighash_none`(`_anyone`) values byte-for-byte.
    #[test]
    fn test_single_without_corresponding_output_uses_empty_outputs_digest() {
        let (tx, spent_utxos, orchard_digest) = official_vector_6();
        assert!(tx.output.is_empty());

        let cases = [
            (
                ZcashSighashType::Single,
                "f039574a6092becc8b3b4987b841584a27bc1af170f59f91139df4c98b57010e",
            ),
            (
                ZcashSighashType::SinglePlusAnyoneCanPay,
                "7f9d479c3766b0551d1761402beccf9cbc171183fc335003441f36fdcb2daa63",
            ),
        ];
        for (sighash_type, expected) in cases {
            let sighash = signature_digest_with_bundle_digests(
                &tx,
                1,
                sighash_type,
                &spent_utxos,
                &empty_sapling_digest(),
                &orchard_digest,
            );
            assert_eq!(hex::encode(sighash), expected, "type {sighash_type:?}");
        }
    }

    #[test]
    #[should_panic(expected = "input_index out of bounds")]
    fn test_sighash_panics_on_input_index_out_of_bounds() {
        let (tx, spent_utxos, orchard_digest) = official_vector_4();
        signature_digest_with_bundle_digests(
            &tx,
            2,
            ZcashSighashType::All,
            &spent_utxos,
            &empty_sapling_digest(),
            &orchard_digest,
        );
    }
}
