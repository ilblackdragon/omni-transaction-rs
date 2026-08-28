//! Sui transaction: BCS encoding, intent-message signing digest and the
//! canonical signed-transaction envelope.
use super::types::{GasData, SuiAddress, SuiSignature, TransactionExpiration, TransactionKind};
use super::utils::blake2b256;
use crate::bcs_encoding::{BcsEncode, BcsWriter};
#[cfg(feature = "schemars")]
use schemars::JsonSchema;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// The signing intent prefixed to the transaction bytes before hashing:
/// scope = `TransactionData` (0), version = `V0` (0), app id = `Sui` (0).
const TRANSACTION_INTENT: [u8; 3] = [0x00, 0x00, 0x00];

/// A Sui transaction (`TransactionData::V1` with a programmable transaction
/// block).
///
/// # Encoding entry points
///
/// - [`Self::tx_bytes`] — BCS bytes of the unsigned transaction; base64 this
///   for the `tx_bytes` parameter of `sui_executeTransactionBlock`.
/// - [`Self::build_for_signing`] — the **final 32-byte** Blake2b-256 signing
///   digest (see the method docs; this differs from the EVM/Bitcoin
///   builders, which return preimages).
/// - [`Self::build_with_signature`] — the canonical self-contained signed
///   transaction (`SenderSignedData` BCS envelope).
///
/// ###### Example:
///
/// ```rust
/// use omni_transaction::sui::types::{
///     Argument, CallArg, Command, GasData, ObjectDigest, ObjectRef,
///     ProgrammableTransaction, TransactionExpiration, TransactionKind,
/// };
/// use omni_transaction::sui::utils::parse_sui_address;
/// use omni_transaction::sui::SuiTransaction;
///
/// let sender = parse_sui_address("0x2");
/// let recipient = parse_sui_address("0x3");
///
/// // Send 1_000_000 MIST from the gas coin to the recipient.
/// let tx = SuiTransaction {
///     kind: TransactionKind::ProgrammableTransaction(ProgrammableTransaction {
///         inputs: vec![
///             CallArg::pure_u64(1_000_000),
///             CallArg::pure_address(recipient),
///         ],
///         commands: vec![
///             Command::SplitCoins {
///                 coin: Argument::GasCoin,
///                 amounts: vec![Argument::Input(0)],
///             },
///             Command::TransferObjects {
///                 objects: vec![Argument::Result(0)],
///                 address: Argument::Input(1),
///             },
///         ],
///     }),
///     sender,
///     gas_data: GasData {
///         payment: vec![ObjectRef::new(
///             parse_sui_address("0x1"),
///             2,
///             ObjectDigest::new([0x63u8; 32]),
///         )],
///         owner: sender,
///         price: 1000,
///         budget: 5_000_000,
///     },
///     expiration: TransactionExpiration::None,
/// };
///
/// let digest = tx.build_for_signing();
/// assert_eq!(digest.len(), 32); // sign these bytes with ed25519
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct SuiTransaction {
    /// What the transaction does: a programmable transaction block.
    pub kind: TransactionKind,
    /// The address sending the transaction. Must equal
    /// `blake2b256(scheme_flag || public_key)` of the signing key (see
    /// [`crate::sui::utils::derive_sui_address`]).
    pub sender: SuiAddress,
    /// Gas coins, owner, price and budget.
    pub gas_data: GasData,
    /// Epoch-based time-to-live of the transaction.
    pub expiration: TransactionExpiration,
}

impl SuiTransaction {
    /// Returns the BCS bytes of the unsigned transaction,
    /// `bcs(TransactionData::V1(..))`.
    ///
    /// Base64 this value for the `tx_bytes` parameter of the
    /// `sui_executeTransactionBlock` / `sui_dryRunTransactionBlock` JSON-RPC
    /// methods (which take the signature separately; see
    /// [`SuiSignature::to_bytes`]).
    pub fn tx_bytes(&self) -> Vec<u8> {
        crate::bcs_encoding::to_bytes(self)
    }

    /// Returns the intent message: `[0x00, 0x00, 0x00] || tx_bytes()`.
    ///
    /// The three intent bytes are scope = `TransactionData`, version = `V0`,
    /// app id = `Sui`. Exposed so clients can independently recompute the
    /// signing digest: `build_for_signing() ==
    /// blake2b256(build_intent_message())`.
    pub fn build_intent_message(&self) -> Vec<u8> {
        let tx_bytes = self.tx_bytes();
        let mut message = Vec::with_capacity(TRANSACTION_INTENT.len() + tx_bytes.len());
        message.extend_from_slice(&TRANSACTION_INTENT);
        message.extend_from_slice(&tx_bytes);
        message
    }

    /// Returns the **final 32-byte Blake2b-256 signing digest** of the
    /// intent message.
    ///
    /// Unlike the EVM and Bitcoin builders — whose `build_for_signing`
    /// returns a *preimage* the caller still hashes — this value is already
    /// the digest, because the NEAR runtime has no blake2b host function.
    ///
    /// - **ed25519** (primary path): pass these 32 bytes verbatim as the
    ///   message to the signer (e.g. NEAR MPC ed25519). Do **not** hash them
    ///   again; Sui validators run `ed25519_verify(digest, signature)`.
    /// - **secp256k1**: Sui ECDSA hashes the message with SHA-256
    ///   internally, so the scalar actually signed is
    ///   `sha256(build_for_signing())` — see `build_secp256k1_sign_hash`
    ///   (requires the `sha2` feature).
    pub fn build_for_signing(&self) -> Vec<u8> {
        blake2b256(&self.build_intent_message()).to_vec()
    }

    /// Returns the 32-byte hash a raw secp256k1 ECDSA signer (one that signs
    /// a caller-supplied hash, such as the NEAR MPC) must sign:
    /// `sha256(blake2b256(intent || tx_bytes))`.
    ///
    /// Sui verifies ECDSA signatures with SHA-256 as the internal hash over
    /// the Blake2b-256 signing digest. The resulting signature must be
    /// 64-byte `r || s`, low-`s` normalized, without a recovery id (see
    /// [`SuiSignature`]).
    ///
    /// Only available with the `sha2` feature (enabled by default through
    /// the `bitcoin` feature).
    #[cfg(feature = "sha2")]
    pub fn build_secp256k1_sign_hash(&self) -> [u8; 32] {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(self.build_for_signing());
        hasher.finalize().into()
    }

    /// Returns the transaction digest as base58 — the identifier explorers
    /// and RPC nodes use to reference the transaction after submission.
    ///
    /// Sui's canonical `TransactionDigest` is
    /// `blake2b256("TransactionData::" || bcs(TransactionData))` — a
    /// type-name domain separator, **not** the signing intent. It therefore
    /// differs from `base58(build_for_signing())`.
    pub fn digest_base58(&self) -> String {
        let mut preimage = b"TransactionData::".to_vec();
        preimage.extend(self.tx_bytes());
        bs58::encode(blake2b256(&preimage)).into_string()
    }

    /// Returns the canonical self-contained signed transaction,
    /// `bcs(SenderSignedData)`:
    ///
    /// ```text
    /// 0x01                    ; ULEB sequence length: exactly 1 element
    /// 0x00 0x00 0x00          ; intent (TransactionData, V0, Sui)
    /// bcs(TransactionData)    ; tx_bytes(), starts with the 0x00 V1 tag
    /// 0x01                    ; ULEB signature count
    /// 0x61 flag sig(64) pk    ; each signature as ULEB-prefixed bytes
    /// ```
    ///
    /// This is the envelope Sui's binary interfaces (gRPC, checkpoint data,
    /// `sui client` raw output) carry. The JSON-RPC
    /// `sui_executeTransactionBlock` method does **not** accept this blob —
    /// it takes `base64(tx_bytes())` and `base64(signature.to_bytes())` as
    /// separate parameters.
    ///
    /// Sponsored transactions (`gas_data.owner != sender`) require both the
    /// sender's and the sponsor's signatures — use
    /// [`Self::build_with_signatures`] for those.
    pub fn build_with_signature(&self, signature: &SuiSignature) -> Vec<u8> {
        self.build_with_signatures(std::slice::from_ref(signature))
    }

    /// Same envelope as [`Self::build_with_signature`], with an explicit
    /// signature list. A sponsored transaction (`gas_data.owner != sender`)
    /// must carry the sender's and the gas owner's signatures; a plain
    /// transaction carries exactly one.
    ///
    /// # Panics
    ///
    /// Panics if `signatures` is empty — validators reject unsigned
    /// transactions, so an empty list is always a caller bug.
    pub fn build_with_signatures(&self, signatures: &[SuiSignature]) -> Vec<u8> {
        assert!(
            !signatures.is_empty(),
            "a Sui transaction requires at least one signature"
        );
        let mut writer = BcsWriter::new();
        // SenderSignedData is a sequence of exactly one element.
        writer.write_len(1);
        writer.write_fixed(&TRANSACTION_INTENT);
        self.bcs_encode(&mut writer);
        // vector<UserSignature>, each length-prefixed inside BCS.
        writer.write_len(signatures.len());
        for signature in signatures {
            signature.bcs_encode(&mut writer);
        }
        writer.into_bytes()
    }

    /// Deserializes a transaction from this crate's JSON representation
    /// (hex addresses, base58 digests, base64 pure values as byte arrays) —
    /// the exact format `serde_json::to_string(&tx)` produces.
    ///
    /// # Errors
    ///
    /// Returns any [`serde_json::Error`] produced during deserialization.
    #[cfg(feature = "serde_json")]
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

impl BcsEncode for SuiTransaction {
    fn bcs_encode(&self, writer: &mut BcsWriter) {
        // TransactionData enum: V1 is the only variant.
        writer.write_variant(0);
        self.kind.bcs_encode(writer);
        self.sender.bcs_encode(writer);
        self.gas_data.bcs_encode(writer);
        self.expiration.bcs_encode(writer);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sui::types::{
        Argument, CallArg, Command, Identifier, ObjectArg, ObjectDigest, ObjectRef,
        ProgrammableMoveCall, ProgrammableTransaction, StructTag, TypeTag,
    };
    use sui_sdk_types as sst;

    /// Spec golden vector V1: send SUI from the gas coin.
    const V1_BCS_HEX: &str = "000002000840420f000000000000200000000000000000000000000000000000000000000000000000000000000003020200010100000101020000010100000000000000000000000000000000000000000000000000000000000000000201000000000000000000000000000000000000000000000000000000000000000102000000000000002063636363636363636363636363636363636363636363636363636363636363630000000000000000000000000000000000000000000000000000000000000002e803000000000000404b4c000000000000";
    const V1_SIGNING_DIGEST_HEX: &str =
        "56bb898ff33187d573e7cc2a0124fe8940c94b74d1dedb5020548eb5b4c87c31";
    const V1_TX_BYTES_BASE64: &str = "AAACAAhAQg8AAAAAAAAgAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAMCAgABAQAAAQECAAABAQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAgEAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAQIAAAAAAAAAIGNjY2NjY2NjY2NjY2NjY2NjY2NjY2NjY2NjY2NjY2NjAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAALoAwAAAAAAAEBLTAAAAAAAAA==";

    /// Spec golden vector V2: `0x2::pay::split<0x2::sui::SUI>` with shared,
    /// owned and pure inputs and an epoch expiration.
    const V2_BCS_HEX: &str = "0000030101000000000000000000000000000000000000000000000000000000000000000601000000000000000101001111111111111111111111111111111111111111111111111111111111111111050000000000000020636363636363636363636363636363636363636363636363636363636363636300082a0000000000000001000000000000000000000000000000000000000000000000000000000000000002037061790573706c69740107000000000000000000000000000000000000000000000000000000000000000203737569035355490002010100010200000000000000000000000000000000000000000000000000000000000000000201000000000000000000000000000000000000000000000000000000000000000102000000000000002063636363636363636363636363636363636363636363636363636363636363630000000000000000000000000000000000000000000000000000000000000002ee020000000000008096980000000000016400000000000000";
    const V2_SIGNING_DIGEST_HEX: &str =
        "4b62d75d483e8720c77a08527eef73d8c52ad7d7d95eb520a6a8794e1e4ea03f";

    /// Spec golden vector V3: Receiving input, MergeCoins, MakeMoveVec with
    /// a type and no elements, two gas coins.
    const V3_BCS_HEX: &str = "000002010222222222222222222222222222222222222222222222222222222222222222220700000000000000204242424242424242424242424242424242424242424242424242424242424242010000000000000000000000000000000000000000000000000000000000000000010200000000000000206363636363636363636363636363636363636363636363636363636363636363020300010101000501020000000000000000000000000000000000000000000000000000000000000000030200000000000000000000000000000000000000000000000000000000000000010200000000000000206363636363636363636363636363636363636363636363636363636363636363333333333333333333333333333333333333333333333333333333333333333309000000000000002042424242424242424242424242424242424242424242424242424242424242420000000000000000000000000000000000000000000000000000000000000003e80300000000000080841e000000000000";
    const V3_SIGNING_DIGEST_HEX: &str =
        "55aa30cf2d2ccf8601f260787b353fd8365a018ef4c13e6ea1c8e6f701ec1ae4";

    fn address(hex: &str) -> SuiAddress {
        SuiAddress::from_hex(hex).unwrap()
    }

    fn ref_address(hex: &str) -> sst::Address {
        sst::Address::from_hex(hex).unwrap()
    }

    fn v1_transaction() -> SuiTransaction {
        SuiTransaction {
            kind: TransactionKind::ProgrammableTransaction(ProgrammableTransaction {
                inputs: vec![
                    CallArg::pure_u64(1_000_000),
                    CallArg::pure_address(address("0x3")),
                ],
                commands: vec![
                    Command::SplitCoins {
                        coin: Argument::GasCoin,
                        amounts: vec![Argument::Input(0)],
                    },
                    Command::TransferObjects {
                        objects: vec![Argument::Result(0)],
                        address: Argument::Input(1),
                    },
                ],
            }),
            sender: address("0x2"),
            gas_data: GasData {
                payment: vec![ObjectRef::new(
                    address("0x1"),
                    2,
                    ObjectDigest::new([0x63; 32]),
                )],
                owner: address("0x2"),
                price: 1000,
                budget: 5_000_000,
            },
            expiration: TransactionExpiration::None,
        }
    }

    fn v1_reference() -> sst::Transaction {
        sst::Transaction {
            kind: sst::TransactionKind::ProgrammableTransaction(sst::ProgrammableTransaction {
                inputs: vec![
                    sst::Input::Pure(1_000_000u64.to_le_bytes().to_vec()),
                    sst::Input::Pure(ref_address("0x3").into_inner().to_vec()),
                ],
                commands: vec![
                    sst::Command::SplitCoins(sst::SplitCoins {
                        coin: sst::Argument::Gas,
                        amounts: vec![sst::Argument::Input(0)],
                    }),
                    sst::Command::TransferObjects(sst::TransferObjects {
                        objects: vec![sst::Argument::Result(0)],
                        address: sst::Argument::Input(1),
                    }),
                ],
            }),
            sender: ref_address("0x2"),
            gas_payment: sst::GasPayment {
                objects: vec![sst::ObjectReference::new(
                    ref_address("0x1"),
                    2,
                    sst::Digest::new([0x63; 32]),
                )],
                owner: ref_address("0x2"),
                price: 1000,
                budget: 5_000_000,
            },
            expiration: sst::TransactionExpiration::None,
        }
    }

    fn v2_transaction() -> SuiTransaction {
        SuiTransaction {
            kind: TransactionKind::ProgrammableTransaction(ProgrammableTransaction {
                inputs: vec![
                    CallArg::Object(ObjectArg::SharedObject {
                        id: address("0x6"),
                        initial_shared_version: 1,
                        mutable: true,
                    }),
                    CallArg::Object(ObjectArg::ImmOrOwnedObject(ObjectRef::new(
                        SuiAddress::new([0x11; 32]),
                        5,
                        ObjectDigest::new([0x63; 32]),
                    ))),
                    CallArg::pure_u64(42),
                ],
                commands: vec![Command::MoveCall(ProgrammableMoveCall {
                    package: address("0x2"),
                    module: Identifier::new("pay").unwrap(),
                    function: Identifier::new("split").unwrap(),
                    type_arguments: vec![TypeTag::Struct(Box::new(StructTag::sui()))],
                    arguments: vec![Argument::Input(1), Argument::Input(2)],
                })],
            }),
            sender: address("0x2"),
            gas_data: GasData {
                payment: vec![ObjectRef::new(
                    address("0x1"),
                    2,
                    ObjectDigest::new([0x63; 32]),
                )],
                owner: address("0x2"),
                price: 750,
                budget: 10_000_000,
            },
            expiration: TransactionExpiration::Epoch(100),
        }
    }

    fn v2_reference() -> sst::Transaction {
        use std::str::FromStr;
        sst::Transaction {
            kind: sst::TransactionKind::ProgrammableTransaction(sst::ProgrammableTransaction {
                inputs: vec![
                    sst::Input::Shared(sst::SharedInput::new(ref_address("0x6"), 1, true)),
                    sst::Input::ImmutableOrOwned(sst::ObjectReference::new(
                        sst::Address::new([0x11; 32]),
                        5,
                        sst::Digest::new([0x63; 32]),
                    )),
                    sst::Input::Pure(42u64.to_le_bytes().to_vec()),
                ],
                commands: vec![sst::Command::MoveCall(sst::MoveCall {
                    package: ref_address("0x2"),
                    module: sst::Identifier::new("pay").unwrap(),
                    function: sst::Identifier::new("split").unwrap(),
                    type_arguments: vec![sst::TypeTag::from_str("0x2::sui::SUI").unwrap()],
                    arguments: vec![sst::Argument::Input(1), sst::Argument::Input(2)],
                })],
            }),
            sender: ref_address("0x2"),
            gas_payment: sst::GasPayment {
                objects: vec![sst::ObjectReference::new(
                    ref_address("0x1"),
                    2,
                    sst::Digest::new([0x63; 32]),
                )],
                owner: ref_address("0x2"),
                price: 750,
                budget: 10_000_000,
            },
            expiration: sst::TransactionExpiration::Epoch(100),
        }
    }

    fn v3_transaction() -> SuiTransaction {
        SuiTransaction {
            kind: TransactionKind::ProgrammableTransaction(ProgrammableTransaction {
                inputs: vec![
                    CallArg::Object(ObjectArg::Receiving(ObjectRef::new(
                        SuiAddress::new([0x22; 32]),
                        7,
                        ObjectDigest::new([0x42; 32]),
                    ))),
                    CallArg::Object(ObjectArg::ImmOrOwnedObject(ObjectRef::new(
                        address("0x1"),
                        2,
                        ObjectDigest::new([0x63; 32]),
                    ))),
                ],
                commands: vec![
                    Command::MergeCoins {
                        coin: Argument::GasCoin,
                        coins_to_merge: vec![Argument::Input(1)],
                    },
                    Command::MakeMoveVec {
                        type_tag: Some(TypeTag::U64),
                        elements: vec![],
                    },
                ],
            }),
            sender: address("0x3"),
            gas_data: GasData {
                payment: vec![
                    ObjectRef::new(address("0x1"), 2, ObjectDigest::new([0x63; 32])),
                    ObjectRef::new(
                        SuiAddress::new([0x33; 32]),
                        9,
                        ObjectDigest::new([0x42; 32]),
                    ),
                ],
                owner: address("0x3"),
                price: 1000,
                budget: 2_000_000,
            },
            expiration: TransactionExpiration::None,
        }
    }

    fn v3_reference() -> sst::Transaction {
        sst::Transaction {
            kind: sst::TransactionKind::ProgrammableTransaction(sst::ProgrammableTransaction {
                inputs: vec![
                    sst::Input::Receiving(sst::ObjectReference::new(
                        sst::Address::new([0x22; 32]),
                        7,
                        sst::Digest::new([0x42; 32]),
                    )),
                    sst::Input::ImmutableOrOwned(sst::ObjectReference::new(
                        ref_address("0x1"),
                        2,
                        sst::Digest::new([0x63; 32]),
                    )),
                ],
                commands: vec![
                    sst::Command::MergeCoins(sst::MergeCoins {
                        coin: sst::Argument::Gas,
                        coins_to_merge: vec![sst::Argument::Input(1)],
                    }),
                    sst::Command::MakeMoveVector(sst::MakeMoveVector {
                        type_: Some(sst::TypeTag::U64),
                        elements: vec![],
                    }),
                ],
            }),
            sender: ref_address("0x3"),
            gas_payment: sst::GasPayment {
                objects: vec![
                    sst::ObjectReference::new(ref_address("0x1"), 2, sst::Digest::new([0x63; 32])),
                    sst::ObjectReference::new(
                        sst::Address::new([0x33; 32]),
                        9,
                        sst::Digest::new([0x42; 32]),
                    ),
                ],
                owner: ref_address("0x3"),
                price: 1000,
                budget: 2_000_000,
            },
            expiration: sst::TransactionExpiration::None,
        }
    }

    fn assert_vector(
        transaction: &SuiTransaction,
        reference: &sst::Transaction,
        bcs_hex: &str,
        digest_hex: &str,
    ) {
        // Golden bytes from the spec.
        assert_eq!(hex::encode(transaction.tx_bytes()), bcs_hex);
        assert_eq!(hex::encode(transaction.build_for_signing()), digest_hex);
        // Byte-parity with the reference implementation.
        assert_eq!(transaction.tx_bytes(), ::bcs::to_bytes(reference).unwrap());
        assert_eq!(
            transaction.build_for_signing(),
            reference.signing_digest().to_vec()
        );
    }

    #[test]
    fn test_v1_split_and_transfer_vector() {
        assert_vector(
            &v1_transaction(),
            &v1_reference(),
            V1_BCS_HEX,
            V1_SIGNING_DIGEST_HEX,
        );
    }

    #[test]
    fn test_v2_move_call_vector() {
        assert_vector(
            &v2_transaction(),
            &v2_reference(),
            V2_BCS_HEX,
            V2_SIGNING_DIGEST_HEX,
        );
    }

    #[test]
    fn test_v3_receiving_merge_make_move_vec_vector() {
        assert_vector(
            &v3_transaction(),
            &v3_reference(),
            V3_BCS_HEX,
            V3_SIGNING_DIGEST_HEX,
        );
    }

    #[test]
    fn test_v1_tx_bytes_base64_for_rpc() {
        use base64::{engine::general_purpose::STANDARD, Engine};
        assert_eq!(
            STANDARD.encode(v1_transaction().tx_bytes()),
            V1_TX_BYTES_BASE64
        );
    }

    #[test]
    fn test_intent_message_layout() {
        let transaction = v1_transaction();
        let message = transaction.build_intent_message();
        assert_eq!(&message[..3], &[0x00, 0x00, 0x00]);
        assert_eq!(&message[3..], transaction.tx_bytes().as_slice());
        assert_eq!(
            blake2b256(&message).to_vec(),
            transaction.build_for_signing()
        );
    }

    #[test]
    fn test_digest_base58_matches_reference_transaction_digest() {
        let transaction = v1_transaction();
        // Independently recompute Sui's canonical TransactionDigest with the
        // reference SDK: blake2b256("TransactionData::" || bcs(tx)), which is
        // NOT the signing digest (blake2b256(intent || bcs(tx))).
        let reference: sui_sdk_types::Transaction =
            ::bcs::from_bytes(&transaction.tx_bytes()).unwrap();
        assert_eq!(transaction.digest_base58(), reference.digest().to_string());
        assert_ne!(
            bs58::decode(transaction.digest_base58())
                .into_vec()
                .unwrap(),
            transaction.build_for_signing(),
            "the explorer digest must not be the signing digest"
        );
    }

    /// Publish and Upgrade commands (no golden hex; reference-crate parity).
    #[test]
    fn test_publish_and_upgrade_against_reference_sdk() {
        let modules = vec![vec![0xca, 0xfe], vec![0xba, 0xbe, 0x01]];
        let dependencies = vec![address("0x1"), address("0x2")];

        let mut transaction = v1_transaction();
        transaction.kind = TransactionKind::ProgrammableTransaction(ProgrammableTransaction {
            inputs: vec![],
            commands: vec![
                Command::Publish {
                    modules: modules.clone(),
                    dependencies: dependencies.clone(),
                },
                Command::Upgrade {
                    modules: modules.clone(),
                    dependencies,
                    package: address("0xdead"),
                    ticket: Argument::Result(0),
                },
            ],
        });

        let mut reference = v1_reference();
        reference.kind =
            sst::TransactionKind::ProgrammableTransaction(sst::ProgrammableTransaction {
                inputs: vec![],
                commands: vec![
                    sst::Command::Publish(sst::Publish {
                        modules: modules.clone(),
                        dependencies: vec![ref_address("0x1"), ref_address("0x2")],
                    }),
                    sst::Command::Upgrade(sst::Upgrade {
                        modules,
                        dependencies: vec![ref_address("0x1"), ref_address("0x2")],
                        package: ref_address("0xdead"),
                        ticket: sst::Argument::Result(0),
                    }),
                ],
            });

        assert_eq!(transaction.tx_bytes(), ::bcs::to_bytes(&reference).unwrap());
        assert_eq!(
            transaction.build_for_signing(),
            reference.signing_digest().to_vec()
        );
    }

    /// Empty inputs/commands and >255 input indices (u16 LE encoding).
    #[test]
    fn test_edge_cases_against_reference_sdk() {
        let mut transaction = v1_transaction();
        transaction.kind = TransactionKind::ProgrammableTransaction(ProgrammableTransaction {
            inputs: vec![],
            commands: vec![],
        });
        let mut reference = v1_reference();
        reference.kind =
            sst::TransactionKind::ProgrammableTransaction(sst::ProgrammableTransaction {
                inputs: vec![],
                commands: vec![],
            });
        assert_eq!(transaction.tx_bytes(), ::bcs::to_bytes(&reference).unwrap());

        let mut transaction = v1_transaction();
        transaction.kind = TransactionKind::ProgrammableTransaction(ProgrammableTransaction {
            inputs: (0..=300).map(|_| CallArg::pure_bool(true)).collect(),
            commands: vec![Command::MergeCoins {
                coin: Argument::GasCoin,
                coins_to_merge: vec![Argument::Input(300)],
            }],
        });
        let mut reference = v1_reference();
        reference.kind =
            sst::TransactionKind::ProgrammableTransaction(sst::ProgrammableTransaction {
                inputs: (0..=300).map(|_| sst::Input::Pure(vec![0x01])).collect(),
                commands: vec![sst::Command::MergeCoins(sst::MergeCoins {
                    coin: sst::Argument::Gas,
                    coins_to_merge: vec![sst::Argument::Input(300)],
                })],
            });
        assert_eq!(transaction.tx_bytes(), ::bcs::to_bytes(&reference).unwrap());
        assert_eq!(
            transaction.build_for_signing(),
            reference.signing_digest().to_vec()
        );
    }

    /// Spec layout test: signed envelope with a synthetic signature.
    #[test]
    fn test_build_with_signature_layout() {
        let transaction = v1_transaction();
        let signature = SuiSignature::ed25519([0x01u8; 64], [0x02u8; 32]);
        let signed = transaction.build_with_signature(&signature);

        let mut expected = vec![0x01, 0x00, 0x00, 0x00];
        expected.extend_from_slice(&transaction.tx_bytes());
        expected.push(0x01); // one signature
        expected.push(0x61); // ULEB length 97
        expected.extend_from_slice(&signature.to_bytes());
        assert_eq!(signed, expected);
    }

    /// Sponsored transactions carry the sender's and the gas owner's
    /// signatures in one envelope.
    #[test]
    fn test_build_with_signatures_layout_for_sponsored_transactions() {
        let transaction = v1_transaction();
        let sender_sig = SuiSignature::ed25519([0x01u8; 64], [0x02u8; 32]);
        let sponsor_sig = SuiSignature::ed25519([0x03u8; 64], [0x04u8; 32]);
        let signed = transaction.build_with_signatures(&[sender_sig.clone(), sponsor_sig.clone()]);

        let mut expected = vec![0x01, 0x00, 0x00, 0x00];
        expected.extend_from_slice(&transaction.tx_bytes());
        expected.push(0x02); // two signatures
        expected.push(0x61); // ULEB length 97
        expected.extend_from_slice(&sender_sig.to_bytes());
        expected.push(0x61);
        expected.extend_from_slice(&sponsor_sig.to_bytes());
        assert_eq!(signed, expected);
    }

    #[test]
    #[should_panic(expected = "at least one signature")]
    fn test_build_with_signatures_rejects_empty_list() {
        v1_transaction().build_with_signatures(&[]);
    }

    /// Sign the digest with a real ed25519 key and compare the whole signed
    /// envelope against the reference SDK's canonical binary serialization.
    #[test]
    fn test_build_with_signature_against_reference_sdk() {
        use ed25519_dalek::{Signer, SigningKey};

        let signing_key = SigningKey::from_bytes(&[0x42u8; 32]);
        let verifying_key = signing_key.verifying_key();

        let transaction = v1_transaction();
        let digest = transaction.build_for_signing();
        let signature = signing_key.sign(&digest);

        let sui_signature = SuiSignature::ed25519(signature.to_bytes(), verifying_key.to_bytes());
        let signed = transaction.build_with_signature(&sui_signature);

        // Reference: seq(1) || intent(0,0,0) || bcs(Transaction) ||
        // bcs(Vec<UserSignature>), per sui-sdk-types
        // SignedTransactionWithIntentMessage.
        let reference_signature = sst::UserSignature::Simple(sst::SimpleSignature::Ed25519 {
            signature: sst::Ed25519Signature::new(signature.to_bytes()),
            public_key: sst::Ed25519PublicKey::new(verifying_key.to_bytes()),
        });
        let mut expected = vec![0x01, 0x00, 0x00, 0x00];
        expected.extend_from_slice(&::bcs::to_bytes(&v1_reference()).unwrap());
        expected.extend_from_slice(&::bcs::to_bytes(&vec![reference_signature]).unwrap());
        assert_eq!(signed, expected);
    }

    #[test]
    #[cfg(feature = "sha2")]
    fn test_secp256k1_sign_hash_is_sha256_of_digest() {
        use sha2::{Digest, Sha256};
        let transaction = v1_transaction();
        let expected: [u8; 32] = Sha256::digest(transaction.build_for_signing()).into();
        assert_eq!(transaction.build_secp256k1_sign_hash(), expected);
    }

    #[test]
    #[cfg(feature = "serde_json")]
    fn test_json_round_trip() {
        let transaction = v2_transaction();
        let json = serde_json::to_string(&transaction).unwrap();
        let back = SuiTransaction::from_json(&json).unwrap();
        assert_eq!(back, transaction);
        assert_eq!(back.tx_bytes(), transaction.tx_bytes());
    }
}
