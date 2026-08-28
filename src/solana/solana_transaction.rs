//! Solana transaction
#[cfg(feature = "schemars")]
use schemars::JsonSchema;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use super::constants::PACKET_DATA_SIZE;
use super::encoding::{compact_u16_len, encode_length};
use super::types::{SolanaMessage, SolanaSignature};
use crate::constants::ED25519_SIGNATURE_LENGTH;

/// A Solana transaction: a [`SolanaMessage`] (legacy or V0) plus, once
/// available, the ed25519 signatures of its required signers.
///
/// [`Self::build_for_signing`] returns the serialized message bytes — the
/// exact ed25519 signing *preimage* (Solana never pre-hashes the payload
/// outside the signature algorithm itself), which is what a NEAR MPC `Eddsa`
/// sign request must carry. [`Self::build_with_signature`] then produces the
/// broadcastable wire bytes.
///
/// Note that Solana fee-payer/signer signatures are **ed25519 only**: the
/// secp256k1 sign-a-32-byte-hash flow used by the other chains in this crate
/// does not apply, and the payer key must be the ed25519 public key whose
/// signature will be supplied.
///
/// The message is fully public, so callers holding a pre-compiled message
/// (e.g. from a swap-aggregator API) can construct the transaction directly
/// without going through the builder:
///
/// ###### Example:
///
/// ```rust
/// use omni_transaction::solana::types::{
///     Blockhash, CompiledInstruction, MessageHeader, SolanaAddress, SolanaMessage,
/// };
/// use omni_transaction::solana::SolanaTransaction;
///
/// let from = SolanaAddress::from_base58("4vJ9JU1bJJE96FWSJKvHsmmFADCg4gpZQff4P3bkLKi").unwrap();
/// let to = SolanaAddress::from_base58("8opHzTAnfzRpPEx21XtnrVTX28YQuCpAjcn1PczScKh").unwrap();
/// let system_program = SolanaAddress([0u8; 32]);
///
/// let solana_tx = SolanaTransaction {
///     message: SolanaMessage::Legacy {
///         header: MessageHeader {
///             num_required_signatures: 1,
///             num_readonly_signed_accounts: 0,
///             num_readonly_unsigned_accounts: 1,
///         },
///         account_keys: vec![from, to, system_program],
///         recent_blockhash: Blockhash([0u8; 32]),
///         instructions: vec![CompiledInstruction {
///             program_id_index: 2,
///             accounts: vec![0, 1],
///             data: hex::decode("0200000040420f0000000000").unwrap(),
///         }],
///     },
/// };
///
/// // The ed25519 signing payload (the serialized message, not a hash):
/// let payload = solana_tx.build_for_signing();
///
/// // If you prefer to start from JSON:
/// # #[cfg(feature = "serde_json")]
/// # let _ = {
/// let json = r#"{
///     "message": {
///         "Legacy": {
///             "header": {
///                 "num_required_signatures": 1,
///                 "num_readonly_signed_accounts": 0,
///                 "num_readonly_unsigned_accounts": 1
///             },
///             "account_keys": [
///                 "4vJ9JU1bJJE96FWSJKvHsmmFADCg4gpZQff4P3bkLKi",
///                 "8opHzTAnfzRpPEx21XtnrVTX28YQuCpAjcn1PczScKh",
///                 "11111111111111111111111111111111"
///             ],
///             "recent_blockhash": "11111111111111111111111111111111",
///             "instructions": [{
///                 "program_id_index": 2,
///                 "accounts": [0, 1],
///                 "data": [2, 0, 0, 0, 64, 66, 15, 0, 0, 0, 0, 0]
///             }]
///         }
///     }
/// }"#;
/// let from_json = SolanaTransaction::from_json(json).unwrap();
/// assert_eq!(from_json.build_for_signing(), payload);
/// # };
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct SolanaTransaction {
    /// The message to sign and broadcast.
    pub message: SolanaMessage,
}

impl SolanaTransaction {
    /// Returns the serialized message bytes: the exact ed25519 signing
    /// preimage (a V0 message includes its `0x80` version prefix).
    ///
    /// Do **not** hash this payload before signing — Solana signatures are
    /// ed25519 over the raw message bytes.
    ///
    /// # Panics
    ///
    /// Panics if the transaction could never be broadcast because the signed
    /// form would exceed
    /// [`PACKET_DATA_SIZE`](super::constants::PACKET_DATA_SIZE) (1232) bytes,
    /// i.e. if [`Self::signed_size`] is above that limit. The check happens
    /// here — *before* any signing round trip — so that an oversized
    /// instruction set is rejected without first paying for (and waiting on)
    /// a signature; [`Self::build_with_signature`] re-checks the final bytes.
    pub fn build_for_signing(&self) -> Vec<u8> {
        let message_bytes = self.message.to_bytes();
        assert_fits_packet_limit(
            self.signed_size_for_message_len(message_bytes.len()),
            "signed transaction (message plus signatures) would be",
        );
        message_bytes
    }

    /// Number of bytes the broadcastable signed transaction will occupy: the
    /// `compact-u16` signature count, plus 64 bytes per required signature,
    /// plus the serialized message.
    ///
    /// Compare against
    /// [`PACKET_DATA_SIZE`](super::constants::PACKET_DATA_SIZE) to check a
    /// transaction without panicking (both [`Self::build_for_signing`] and
    /// [`Self::build_with_signature`] enforce that limit).
    pub fn signed_size(&self) -> usize {
        self.signed_size_for_message_len(self.message.to_bytes().len())
    }

    /// Panics if the signed transaction would exceed
    /// [`PACKET_DATA_SIZE`](super::constants::PACKET_DATA_SIZE); used by the
    /// builder to reject an unbroadcastable transaction at compile time.
    pub(crate) fn assert_within_packet_limit(&self) {
        assert_fits_packet_limit(
            self.signed_size(),
            "signed transaction (message plus signatures) would be",
        );
    }

    /// [`Self::signed_size`] for an already-serialized message of
    /// `message_len` bytes.
    fn signed_size_for_message_len(&self, message_len: usize) -> usize {
        let signatures = self.message.num_required_signatures();
        compact_u16_len(u16::from(signatures))
            + usize::from(signatures) * ED25519_SIGNATURE_LENGTH
            + message_len
    }

    /// Returns the broadcastable signed-transaction wire bytes:
    /// `compact-u16` signature count, then `count * 64` raw signature bytes,
    /// then the message bytes exactly as produced by
    /// [`Self::build_for_signing`].
    ///
    /// `signatures[i]` must be the signature of `account_keys[i]` (the caller
    /// is responsible for that ordering when there are multiple signers).
    /// A client then base64-encodes the returned bytes for JSON-RPC
    /// `sendTransaction`; the transaction id is `base58(signature[0])`.
    ///
    /// # Panics
    ///
    /// Panics if `signatures.len()` differs from the message header's
    /// `num_required_signatures`, or if the serialized transaction exceeds
    /// [`PACKET_DATA_SIZE`](super::constants::PACKET_DATA_SIZE) (1232) bytes
    /// — validators reject anything larger, so an oversized transaction is
    /// always a bug in the caller's instruction set. That size limit is
    /// already enforced by [`Self::build_for_signing`] (and by the builder),
    /// so it should never first surface here, in a signature callback.
    pub fn build_with_signature(&self, signatures: &[SolanaSignature]) -> Vec<u8> {
        let required = usize::from(self.message.num_required_signatures());
        assert_eq!(
            signatures.len(),
            required,
            "expected {} signature(s), got {}",
            required,
            signatures.len()
        );

        let message_bytes = self.message.to_bytes();
        let mut out = Vec::with_capacity(3 + signatures.len() * 64 + message_bytes.len());
        encode_length(signatures.len(), &mut out);
        for signature in signatures {
            out.extend_from_slice(&signature.0);
        }
        out.extend_from_slice(&message_bytes);
        assert_fits_packet_limit(out.len(), "serialized transaction is");
        out
    }

    /// Builds a `SolanaTransaction` from a JSON string (see the type-level
    /// example for the expected shape; addresses, blockhashes and signatures
    /// are base58 strings or byte arrays).
    #[cfg(feature = "serde_json")]
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

/// Panics if `size` exceeds Solana's packet limit; `subject` completes the
/// sentence "`{subject} {size} bytes, above ...`".
fn assert_fits_packet_limit(size: usize, subject: &str) {
    assert!(
        size <= PACKET_DATA_SIZE,
        "{subject} {size} bytes, above Solana's {PACKET_DATA_SIZE}-byte packet limit"
    );
}

#[cfg(test)]
mod tests {
    use ed25519_dalek::Signer as DalekSigner;

    use super::*;
    use crate::solana::types::{
        Blockhash, CompiledInstruction, MessageAddressTableLookup, MessageHeader, SolanaAddress,
    };

    /// Spec vector V1: legacy System transfer of 1_000_000 lamports with an
    /// all-zero blockhash.
    const V1_MESSAGE_HEX: &str = "01000103010101010101010101010101010101010101010101010101010101010101010102000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001020200010c0200000040420f0000000000";

    /// Spec vector V2: legacy transfer of 42 lamports signed by the keypair
    /// derived from seed `[0x01; 32]`.
    const V2_MESSAGE_HEX: &str = "010001038a88e3dd7409f195fd52db2d3cba5d72ca6709bf1d94121bf3748801b40f6f5c02000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000c49ae77603782054f17a9decea43b444eba0edb12c6f1d31c6e0e4a84bf052eb01020200010c020000002a00000000000000";
    const V2_SIGNATURE_HEX: &str = "a8038ea08a399e279d692b0a5710e60f9dbe8781f9b0cf444d8be892126a5a9893097f056a88c75b126ad9ea78a5133be308b8bc46c424ab912d93e6e3273c0b";

    /// Spec vector V4: V0 message with one address table lookup, signed by
    /// the same seed keypair.
    const V4_MESSAGE_HEX: &str = "80010001028a88e3dd7409f195fd52db2d3cba5d72ca6709bf1d94121bf3748801b40f6f5c054a535a992921064d24e87160da387c7c35b5ddbc92bb81e41fa8404105448dc49ae77603782054f17a9decea43b444eba0edb12c6f1d31c6e0e4a84bf052eb0101030203000301020301d09e258ae6cf647b0e2441b43818bb9713c47f6351f87bef581f6f8346c9cc2a01000101";
    const V4_SIGNATURE_HEX: &str = "30eea303d15b37c1fc678c9e87b4682b9ec0f526548c81fab3156f36e9e149c7dda33dd3e29df440fbc00779f03697ebd0a6c4e1198d2c308dc87c4a6c8f090f";

    fn address(hex_str: &str) -> SolanaAddress {
        SolanaAddress(hex::decode(hex_str).unwrap().try_into().unwrap())
    }

    fn v1_transaction() -> SolanaTransaction {
        SolanaTransaction {
            message: SolanaMessage::Legacy {
                header: MessageHeader {
                    num_required_signatures: 1,
                    num_readonly_signed_accounts: 0,
                    num_readonly_unsigned_accounts: 1,
                },
                account_keys: vec![
                    SolanaAddress([0x01; 32]),
                    address("0200000000000000000000000000000000000000000000000000000000000000"),
                    SolanaAddress([0x00; 32]),
                ],
                recent_blockhash: Blockhash([0u8; 32]),
                instructions: vec![CompiledInstruction {
                    program_id_index: 2,
                    accounts: vec![0, 1],
                    data: hex::decode("0200000040420f0000000000").unwrap(),
                }],
            },
        }
    }

    fn v2_transaction() -> SolanaTransaction {
        SolanaTransaction {
            message: SolanaMessage::Legacy {
                header: MessageHeader {
                    num_required_signatures: 1,
                    num_readonly_signed_accounts: 0,
                    num_readonly_unsigned_accounts: 1,
                },
                account_keys: vec![
                    address("8a88e3dd7409f195fd52db2d3cba5d72ca6709bf1d94121bf3748801b40f6f5c"),
                    address("0200000000000000000000000000000000000000000000000000000000000000"),
                    SolanaAddress([0x00; 32]),
                ],
                recent_blockhash: Blockhash(
                    hex::decode("c49ae77603782054f17a9decea43b444eba0edb12c6f1d31c6e0e4a84bf052eb")
                        .unwrap()
                        .try_into()
                        .unwrap(),
                ),
                instructions: vec![CompiledInstruction {
                    program_id_index: 2,
                    accounts: vec![0, 1],
                    data: hex::decode("020000002a00000000000000").unwrap(),
                }],
            },
        }
    }

    fn v4_transaction() -> SolanaTransaction {
        SolanaTransaction {
            message: SolanaMessage::V0 {
                header: MessageHeader {
                    num_required_signatures: 1,
                    num_readonly_signed_accounts: 0,
                    num_readonly_unsigned_accounts: 1,
                },
                account_keys: vec![
                    address("8a88e3dd7409f195fd52db2d3cba5d72ca6709bf1d94121bf3748801b40f6f5c"),
                    address("054a535a992921064d24e87160da387c7c35b5ddbc92bb81e41fa8404105448d"),
                ],
                recent_blockhash: Blockhash(
                    hex::decode("c49ae77603782054f17a9decea43b444eba0edb12c6f1d31c6e0e4a84bf052eb")
                        .unwrap()
                        .try_into()
                        .unwrap(),
                ),
                instructions: vec![CompiledInstruction {
                    program_id_index: 1,
                    accounts: vec![2, 3, 0],
                    data: vec![1, 2, 3],
                }],
                address_table_lookups: vec![MessageAddressTableLookup {
                    account_key: address(
                        "d09e258ae6cf647b0e2441b43818bb9713c47f6351f87bef581f6f8346c9cc2a",
                    ),
                    writable_indexes: vec![0],
                    readonly_indexes: vec![1],
                }],
            },
        }
    }

    #[test]
    fn test_v1_build_for_signing_matches_official_vector() {
        assert_eq!(
            hex::encode(v1_transaction().build_for_signing()),
            V1_MESSAGE_HEX
        );
    }

    #[test]
    fn test_v1_build_with_placeholder_signature_matches_official_vector() {
        let wire = v1_transaction().build_with_signature(&[SolanaSignature([0u8; 64])]);
        let expected = format!("01{}{}", "00".repeat(64), V1_MESSAGE_HEX);
        assert_eq!(hex::encode(wire), expected);
    }

    #[test]
    fn test_v2_ed25519_dalek_round_trip_matches_official_vector() {
        let tx = v2_transaction();
        let payload = tx.build_for_signing();
        assert_eq!(hex::encode(&payload), V2_MESSAGE_HEX);

        // Sign the exact preimage with ed25519-dalek from the spec seed.
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&[1u8; 32]);
        assert_eq!(
            signing_key.verifying_key().to_bytes(),
            tx.message.account_keys()[0].to_bytes(),
            "payer must be the seed keypair's public key"
        );
        let signature = signing_key.sign(&payload);
        assert_eq!(hex::encode(signature.to_bytes()), V2_SIGNATURE_HEX);

        let wire = tx.build_with_signature(&[SolanaSignature(signature.to_bytes())]);
        let expected = format!("01{V2_SIGNATURE_HEX}{V2_MESSAGE_HEX}");
        assert_eq!(hex::encode(wire), expected);
    }

    #[test]
    fn test_v4_v0_ed25519_dalek_round_trip_matches_official_vector() {
        let tx = v4_transaction();
        let payload = tx.build_for_signing();
        assert_eq!(hex::encode(&payload), V4_MESSAGE_HEX);

        let signing_key = ed25519_dalek::SigningKey::from_bytes(&[1u8; 32]);
        let signature = signing_key.sign(&payload);
        assert_eq!(hex::encode(signature.to_bytes()), V4_SIGNATURE_HEX);

        let wire = tx.build_with_signature(&[SolanaSignature(signature.to_bytes())]);
        let expected = format!("01{V4_SIGNATURE_HEX}{V4_MESSAGE_HEX}");
        assert_eq!(hex::encode(wire), expected);
    }

    #[test]
    #[should_panic(expected = "expected 1 signature(s), got 2")]
    fn test_build_with_signature_rejects_wrong_signature_count() {
        let tx = v1_transaction();
        tx.build_with_signature(&[SolanaSignature([0u8; 64]), SolanaSignature([1u8; 64])]);
    }

    /// V1 legacy transfer whose single instruction carries `len` data bytes.
    fn legacy_tx_with_data_len(len: usize) -> SolanaTransaction {
        let mut tx = v1_transaction();
        if let SolanaMessage::Legacy { instructions, .. } = &mut tx.message {
            instructions[0].data = vec![0u8; len];
        }
        tx
    }

    /// Largest instruction-data length whose signed transaction still fits
    /// in a packet (found by probing `signed_size`, which never panics).
    fn largest_fitting_data_len() -> usize {
        (0..PACKET_DATA_SIZE)
            .take_while(|&len| legacy_tx_with_data_len(len).signed_size() <= PACKET_DATA_SIZE)
            .last()
            .expect("even an empty instruction must fit")
    }

    /// `signed_size` must predict exactly what `build_with_signature`
    /// produces, for every message shape.
    #[test]
    fn test_signed_size_matches_built_wire_length() {
        for tx in [v1_transaction(), v2_transaction(), v4_transaction()] {
            let wire = tx.build_with_signature(&[SolanaSignature([0u8; 64])]);
            assert_eq!(tx.signed_size(), wire.len());
        }
    }

    /// The packet limit is enforced *before* signing: an oversized
    /// instruction set must be rejected by `build_for_signing`, not only in
    /// the signature callback.
    #[test]
    #[should_panic(expected = "packet limit")]
    fn test_build_for_signing_rejects_oversized_transaction() {
        legacy_tx_with_data_len(2000).build_for_signing();
    }

    /// One byte over the limit: the *message* alone is still below 1232
    /// bytes, so only counting the signature bytes catches this — exactly the
    /// case that used to reach `build_with_signature` (post-MPC) instead.
    #[test]
    #[should_panic(expected = "packet limit")]
    fn test_build_for_signing_rejects_one_byte_over_limit() {
        let over = legacy_tx_with_data_len(largest_fitting_data_len() + 1);
        assert!(
            over.build_for_signing().len() < PACKET_DATA_SIZE,
            "the message alone must still be under the limit for this test to be meaningful"
        );
    }

    /// A transaction exactly at the limit still works end to end.
    #[test]
    fn test_transaction_at_packet_limit_round_trips() {
        let tx = legacy_tx_with_data_len(largest_fitting_data_len());
        assert_eq!(tx.signed_size(), PACKET_DATA_SIZE);
        let message_bytes = tx.build_for_signing();
        assert_eq!(message_bytes, tx.message.to_bytes());
        let wire = tx.build_with_signature(&[SolanaSignature([7u8; 64])]);
        assert_eq!(wire.len(), PACKET_DATA_SIZE);
        assert!(wire.ends_with(&message_bytes));
    }

    #[test]
    #[should_panic(expected = "packet limit")]
    fn test_build_with_signature_rejects_oversized_transaction() {
        legacy_tx_with_data_len(2000).build_with_signature(&[SolanaSignature([0u8; 64])]);
    }

    #[test]
    #[cfg(feature = "serde_json")]
    fn test_from_json_legacy_matches_official_vector() {
        let json = r#"{
            "message": {
                "Legacy": {
                    "header": {
                        "num_required_signatures": 1,
                        "num_readonly_signed_accounts": 0,
                        "num_readonly_unsigned_accounts": 1
                    },
                    "account_keys": [
                        "4vJ9JU1bJJE96FWSJKvHsmmFADCg4gpZQff4P3bkLKi",
                        "8opHzTAnfzRpPEx21XtnrVTX28YQuCpAjcn1PczScKh",
                        "11111111111111111111111111111111"
                    ],
                    "recent_blockhash": "11111111111111111111111111111111",
                    "instructions": [{
                        "program_id_index": 2,
                        "accounts": [0, 1],
                        "data": [2, 0, 0, 0, 64, 66, 15, 0, 0, 0, 0, 0]
                    }]
                }
            }
        }"#;

        let tx = SolanaTransaction::from_json(json).unwrap();
        assert_eq!(hex::encode(tx.build_for_signing()), V1_MESSAGE_HEX);
    }

    #[test]
    #[cfg(feature = "serde_json")]
    fn test_serde_round_trip() {
        for tx in [v1_transaction(), v2_transaction(), v4_transaction()] {
            let json = serde_json::to_string(&tx).unwrap();
            let parsed = SolanaTransaction::from_json(&json).unwrap();
            assert_eq!(parsed, tx);
            assert_eq!(parsed.build_for_signing(), tx.build_for_signing());
        }
    }
}
