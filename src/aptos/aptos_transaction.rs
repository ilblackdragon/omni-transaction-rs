//! Aptos transaction (`RawTransaction`) and its signing/broadcast encodings.
use sha3::{Digest, Sha3_256};

use super::constants::APTOS_RAW_TRANSACTION_SALT;
use super::types::{
    AccountAddress, Ed25519PublicKey, Ed25519Signature, TransactionAuthenticator,
    TransactionPayload,
};
use crate::bcs_encoding::{to_bytes, BcsEncode, BcsWriter};
#[cfg(feature = "schemars")]
use schemars::JsonSchema;
#[cfg(feature = "serde")]
use serde::{Deserialize, Deserializer, Serialize};

/// An Aptos transaction, byte-compatible with aptos-core's `RawTransaction`
/// (`types/src/transaction/mod.rs`) — the field order is the BCS wire format.
///
/// - [`build_for_signing`](Self::build_for_signing) returns the Ed25519
///   signing message: `sha3_256("APTOS::RawTransaction") || bcs(raw_txn)`.
/// - [`build_with_signature`](Self::build_with_signature) returns the BCS
///   `SignedTransaction` bytes to broadcast via
///   `POST /v1/transactions` with
///   `Content-Type: application/x.aptos.signed_transaction+bcs`.
///
/// In JSON (the `serde` feature) the `u64` fields are **emitted as decimal
/// strings** and accepted as either numbers or strings, matching the Aptos
/// REST API: a bare JSON number above 2^53 does not survive a JavaScript
/// `JSON.parse`/`JSON.stringify` round trip, which would silently change the
/// signing message. The BCS encoding is unaffected.
///
/// ###### Example:
///
/// ```rust
/// use omni_transaction::aptos::types::{
///     AccountAddress, EntryFunction, Identifier, ModuleId, TransactionPayload,
/// };
/// use omni_transaction::aptos::AptosTransaction;
///
/// let receiver = AccountAddress::from_hex("0xdd").unwrap();
/// let amount_arg = 1_000u64.to_le_bytes().to_vec();
///
/// let aptos_tx = AptosTransaction {
///     sender: AccountAddress::from_hex("0xa550c18").unwrap(),
///     sequence_number: 0,
///     payload: TransactionPayload::EntryFunction(EntryFunction::new(
///         ModuleId::new(AccountAddress::ONE, Identifier::new("aptos_account").unwrap()),
///         Identifier::new("transfer").unwrap(),
///         vec![],
///         vec![receiver.as_bytes().to_vec(), amount_arg],
///     )),
///     max_gas_amount: 2000,
///     gas_unit_price: 100,
///     expiration_timestamp_secs: 1_735_689_600,
///     chain_id: 1,
/// };
///
/// let signing_message = aptos_tx.build_for_signing();
/// assert_eq!(&signing_message[..32], hex::decode("b5e97db07fa0bd0e5598aa3643a9bc6f6693bddc1a9fec9e674a461eaa00b193").unwrap().as_slice());
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct AptosTransaction {
    /// The sender's account address (32 raw bytes on the wire).
    pub sender: AccountAddress,
    /// The sequence number of the sender's account.
    #[cfg_attr(
        feature = "serde",
        serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")
    )]
    #[cfg_attr(feature = "schemars", schemars(with = "String"))]
    pub sequence_number: u64,
    /// What the transaction executes.
    pub payload: TransactionPayload,
    /// Maximal gas units to spend for this transaction.
    #[cfg_attr(
        feature = "serde",
        serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")
    )]
    #[cfg_attr(feature = "schemars", schemars(with = "String"))]
    pub max_gas_amount: u64,
    /// Price per gas unit, in octas.
    #[cfg_attr(
        feature = "serde",
        serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")
    )]
    #[cfg_attr(feature = "schemars", schemars(with = "String"))]
    pub gas_unit_price: u64,
    /// Expiration as unix seconds; must be in the future when executed.
    #[cfg_attr(
        feature = "serde",
        serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")
    )]
    #[cfg_attr(feature = "schemars", schemars(with = "String"))]
    pub expiration_timestamp_secs: u64,
    /// Chain id for replay protection (mainnet = 1, testnet = 2).
    pub chain_id: u8,
}

impl BcsEncode for AptosTransaction {
    fn bcs_encode(&self, writer: &mut BcsWriter) {
        self.sender.bcs_encode(writer);
        writer.write_u64(self.sequence_number);
        self.payload.bcs_encode(writer);
        writer.write_u64(self.max_gas_amount);
        writer.write_u64(self.gas_unit_price);
        writer.write_u64(self.expiration_timestamp_secs);
        writer.write_u8(self.chain_id);
    }
}

impl AptosTransaction {
    /// Returns the Ed25519 signing message (the full preimage, **not** a
    /// digest):
    ///
    /// `sha3_256("APTOS::RawTransaction") || bcs(raw_transaction)`
    ///
    /// The 32-byte prefix is the SHA3-256 (FIPS-202, not keccak) hash of the
    /// ASCII salt. The signer signs these bytes directly with Ed25519 —
    /// nobody hashes them again (NEAR MPC's ed25519 domain signs them
    /// verbatim). A signature over the unprefixed BCS bytes is invalid
    /// on-chain.
    pub fn build_for_signing(&self) -> Vec<u8> {
        let mut out = Sha3_256::digest(APTOS_RAW_TRANSACTION_SALT).to_vec();
        out.extend_from_slice(&to_bytes(self));
        out
    }

    /// Returns the raw BCS bytes of the `RawTransaction`, **without** the
    /// signing salt prefix. Broadcast bytes embed exactly these.
    pub fn to_bcs_bytes(&self) -> Vec<u8> {
        to_bytes(self)
    }

    /// Returns the broadcast-ready BCS `SignedTransaction` bytes:
    /// `bcs(raw_txn) || 0x00 || 0x20 || public_key(32) || 0x40 || signature(64)`.
    ///
    /// `signature` must be the 64-byte Ed25519 signature over
    /// [`build_for_signing`](Self::build_for_signing)'s output. Broadcast the
    /// returned bytes as-is (no hex/base64/JSON wrapper) via
    /// `POST {fullnode}/v1/transactions` with
    /// `Content-Type: application/x.aptos.signed_transaction+bcs`.
    pub fn build_with_signature(
        &self,
        public_key: &Ed25519PublicKey,
        signature: &Ed25519Signature,
    ) -> Vec<u8> {
        self.build_with_authenticator(&TransactionAuthenticator::Ed25519 {
            public_key: *public_key,
            signature: *signature,
        })
    }

    /// Returns the BCS `SignedTransaction` bytes for an arbitrary
    /// [`TransactionAuthenticator`]: `bcs(raw_txn) || bcs(authenticator)`.
    pub fn build_with_authenticator(&self, authenticator: &TransactionAuthenticator) -> Vec<u8> {
        let mut out = to_bytes(self);
        out.extend_from_slice(&to_bytes(authenticator));
        out
    }

    /// Builds an [`AptosTransaction`] from its JSON representation.
    ///
    /// Addresses, keys and signatures are `0x`-prefixed hex strings; `u64`
    /// fields accept either JSON numbers or decimal strings (serialization
    /// always emits decimal strings, so a round trip is lossless even for
    /// values above 2^53).
    ///
    /// # Errors
    ///
    /// Returns a [`serde_json::Error`] if the JSON is malformed or a field
    /// fails validation.
    ///
    /// ###### Example:
    ///
    /// ```rust
    /// # #[cfg(feature = "serde_json")]
    /// # {
    /// use omni_transaction::aptos::AptosTransaction;
    ///
    /// let json = r#"{
    ///     "sender": "0xa550c18",
    ///     "sequence_number": "0",
    ///     "payload": {
    ///         "EntryFunction": {
    ///             "module": { "address": "0x1", "name": "aptos_account" },
    ///             "function": "transfer",
    ///             "ty_args": [],
    ///             "args": [[221], [1, 0, 0, 0, 0, 0, 0, 0]]
    ///         }
    ///     },
    ///     "max_gas_amount": 2000,
    ///     "gas_unit_price": 100,
    ///     "expiration_timestamp_secs": "1735689600",
    ///     "chain_id": 1
    /// }"#;
    ///
    /// let aptos_tx = AptosTransaction::from_json(json).unwrap();
    /// assert_eq!(aptos_tx.sequence_number, 0);
    /// # }
    /// ```
    #[cfg(feature = "serde_json")]
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

/// Serializes a `u64` as a decimal string.
///
/// The counterpart of [`deserialize_u64`]: JSON numbers lose precision above
/// 2^53, so emitting `18446744073709551615` as a number lets a JavaScript
/// client's `JSON.parse`/`JSON.stringify` round trip turn it into
/// `18446744073709552000` and sign a different transaction.
#[cfg(feature = "serde")]
fn serialize_u64<S>(value: &u64, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.collect_str(value)
}

/// Deserializes a `u64` from either a JSON number or a decimal string
/// (JSON numbers lose precision above 2^53 in JavaScript clients).
#[cfg(feature = "serde")]
fn deserialize_u64<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error as DeError;

    struct U64FlexibleVisitor;

    impl serde::de::Visitor<'_> for U64FlexibleVisitor {
        type Value = u64;

        fn expecting(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
            formatter.write_str("a u64 or a string representing a u64")
        }

        fn visit_u64<E: DeError>(self, v: u64) -> Result<Self::Value, E> {
            Ok(v)
        }

        fn visit_str<E: DeError>(self, s: &str) -> Result<Self::Value, E> {
            s.parse::<u64>()
                .map_err(|_| DeError::custom(format!("invalid u64 string: {s}")))
        }
    }

    deserializer.deserialize_any(U64FlexibleVisitor)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aptos::types::{
        EntryFunction, Identifier, ModuleId, Script, StructTag, TransactionArgument, TypeTag,
    };
    use ed25519_dalek::{Signer, SigningKey, Verifier};

    /// Official TS SDK test key (aptos-core @ aptos-node-v1.5.0,
    /// `transaction_builder.test.ts`).
    const SEED_HEX: &str = "9bf49a6a0755f953811fce125f2683d50429c3bb49e074147e0089a52eae155f";
    const PUBLIC_KEY_HEX: &str = "b9c6ee1630ef3e711144a648db06bbb2284f7274cfbee53ffcee503cc1a49200";

    /// Signing-message vector: prefix || BCS for
    /// 0x1222::aptos_coin::transfer(addr 0xdd, u64 1).
    const SIGNING_MESSAGE_VECTOR: &str = "b5e97db07fa0bd0e5598aa3643a9bc6f6693bddc1a9fec9e674a461eaa00b193000000000000000000000000000000000000000000000000000000000a550c1800000000000000000200000000000000000000000000000000000000000000000000000000000012220a6170746f735f636f696e087472616e7366657200022000000000000000000000000000000000000000000000000000000000000000dd080100000000000000d0070000000000000000000000000000ffffffffffffffff04";

    /// Full SignedTransaction vector for the same raw transaction.
    const SIGNED_TXN_VECTOR: &str = "000000000000000000000000000000000000000000000000000000000a550c1800000000000000000200000000000000000000000000000000000000000000000000000000000012220a6170746f735f636f696e087472616e7366657200022000000000000000000000000000000000000000000000000000000000000000dd080100000000000000d0070000000000000000000000000000ffffffffffffffff040020b9c6ee1630ef3e711144a648db06bbb2284f7274cfbee53ffcee503cc1a49200409c570996380897f38b8d7008d726fb45d6ded0689216e56b73f523492cba92deb6671c27e9a44d2a6fdfdb497420d00c621297a23d6d0298895e0d58cff6060c";

    /// SignedTransaction vector for
    /// 0x1222::coin::transfer<0x1::aptos_coin::AptosCoin>(addr 0xdd, u64 1).
    const SIGNED_TXN_WITH_TY_ARGS_VECTOR: &str = "000000000000000000000000000000000000000000000000000000000a550c18000000000000000002000000000000000000000000000000000000000000000000000000000000122204636f696e087472616e73666572010700000000000000000000000000000000000000000000000000000000000000010a6170746f735f636f696e094170746f73436f696e00022000000000000000000000000000000000000000000000000000000000000000dd080100000000000000d0070000000000000000000000000000ffffffffffffffff040020b9c6ee1630ef3e711144a648db06bbb2284f7274cfbee53ffcee503cc1a4920040112162f543ca92b4f14c1b09b7f52894a127f5428b0d407c09c8efb3a136cff50e550aea7da1226f02571d79230b80bd79096ea0d796789ad594b8fbde695404";

    /// SignedTransaction vector for a script payload with a type argument and
    /// a U8 argument.
    const SIGNED_SCRIPT_VECTOR: &str = "000000000000000000000000000000000000000000000000000000000a550c1800000000000000000026a11ceb0b030000000105000100000000050601000000000000000600000000000000001a0102010700000000000000000000000000000000000000000000000000000000000000010a6170746f735f636f696e094170746f73436f696e00010002d0070000000000000000000000000000ffffffffffffffff040020b9c6ee1630ef3e711144a648db06bbb2284f7274cfbee53ffcee503cc1a49200409936b8d22cec685e720761f6c6135e020911f1a26e220e2a0f3317f5a68942531987259ac9e8688158c77df3e7136637056047d9524edad88ee45d61a9346602";

    /// SignedTransaction vector for a script payload with u16/u32/u256 args.
    const SIGNED_SCRIPT_NEW_INTS_VECTOR: &str = "000000000000000000000000000000000000000000000000000000000a550c180000000000000000000000030611f107111111f10811111111111111111111111111111111111111111111111111111111111111f1d0070000000000000000000000000000ffffffffffffffff040020b9c6ee1630ef3e711144a648db06bbb2284f7274cfbee53ffcee503cc1a49200409402b773f66cf5444efe4de38a026cf9b34e0327798ea01f0695db8e8e0888e20387b08f504b620dcffbc382e3ac141c0ec9a820c5f58b5da2eec589a9e86b0b";

    fn sender() -> AccountAddress {
        AccountAddress::from_hex("0xa550c18").unwrap()
    }

    fn base_tx(payload: TransactionPayload) -> AptosTransaction {
        AptosTransaction {
            sender: sender(),
            sequence_number: 0,
            payload,
            max_gas_amount: 2000,
            gas_unit_price: 0,
            expiration_timestamp_secs: u64::MAX,
            chain_id: 4,
        }
    }

    fn transfer_args() -> Vec<Vec<u8>> {
        let receiver = AccountAddress::from_hex("0xdd").unwrap();
        vec![to_bytes(&receiver), 1u64.to_le_bytes().to_vec()]
    }

    fn signing_key() -> SigningKey {
        let mut seed = [0u8; 32];
        hex::decode_to_slice(SEED_HEX, &mut seed).unwrap();
        SigningKey::from_bytes(&seed)
    }

    /// Signs the transaction with the official test seed, checks the derived
    /// public key, and returns the broadcast bytes.
    fn sign_and_build(tx: &AptosTransaction) -> Vec<u8> {
        let key = signing_key();
        let public_key = Ed25519PublicKey::new(key.verifying_key().to_bytes());
        assert_eq!(public_key.to_hex(), format!("0x{PUBLIC_KEY_HEX}"));
        let signature = Ed25519Signature::new(key.sign(&tx.build_for_signing()).to_bytes());
        tx.build_with_signature(&public_key, &signature)
    }

    #[test]
    fn test_salt_prefix_is_sha3_256_of_apt_raw_transaction() {
        let prefix = Sha3_256::digest(APTOS_RAW_TRANSACTION_SALT);
        assert_eq!(
            hex::encode(prefix),
            "b5e97db07fa0bd0e5598aa3643a9bc6f6693bddc1a9fec9e674a461eaa00b193"
        );
    }

    #[test]
    fn test_build_for_signing_matches_official_vector() {
        let tx = base_tx(TransactionPayload::EntryFunction(EntryFunction::new(
            ModuleId::new(
                AccountAddress::from_hex("0x1222").unwrap(),
                Identifier::new("aptos_coin").unwrap(),
            ),
            Identifier::new("transfer").unwrap(),
            vec![],
            transfer_args(),
        )));
        assert_eq!(hex::encode(tx.build_for_signing()), SIGNING_MESSAGE_VECTOR);
        // The unprefixed BCS bytes are the signing message minus the salt.
        assert_eq!(
            hex::encode(tx.to_bcs_bytes()),
            &SIGNING_MESSAGE_VECTOR[64..]
        );
    }

    #[test]
    fn test_build_with_signature_matches_official_vector() {
        let tx = base_tx(TransactionPayload::EntryFunction(EntryFunction::new(
            ModuleId::new(
                AccountAddress::from_hex("0x1222").unwrap(),
                Identifier::new("aptos_coin").unwrap(),
            ),
            Identifier::new("transfer").unwrap(),
            vec![],
            transfer_args(),
        )));
        // Ed25519 is deterministic: re-deriving the signature from the
        // published seed must reproduce the official vector byte-for-byte.
        assert_eq!(hex::encode(sign_and_build(&tx)), SIGNED_TXN_VECTOR);

        // The same bytes must come out of the pre-built authenticator path.
        let public_key = Ed25519PublicKey::from_hex(PUBLIC_KEY_HEX).unwrap();
        let signature =
            Ed25519Signature::from_hex(&SIGNED_TXN_VECTOR[SIGNED_TXN_VECTOR.len() - 128..])
                .unwrap();
        let authenticator = TransactionAuthenticator::Ed25519 {
            public_key,
            signature,
        };
        assert_eq!(
            hex::encode(tx.build_with_authenticator(&authenticator)),
            SIGNED_TXN_VECTOR
        );
    }

    #[test]
    fn test_entry_function_with_type_args_matches_official_vector() {
        let tx = base_tx(TransactionPayload::EntryFunction(EntryFunction::new(
            ModuleId::new(
                AccountAddress::from_hex("0x1222").unwrap(),
                Identifier::new("coin").unwrap(),
            ),
            Identifier::new("transfer").unwrap(),
            vec![TypeTag::Struct(Box::new(StructTag::aptos_coin()))],
            transfer_args(),
        )));
        assert_eq!(
            hex::encode(sign_and_build(&tx)),
            SIGNED_TXN_WITH_TY_ARGS_VECTOR
        );
    }

    #[test]
    fn test_script_with_type_arg_and_u8_arg_matches_official_vector() {
        let code = hex::decode(
            "a11ceb0b030000000105000100000000050601000000000000000600000000000000001a0102",
        )
        .unwrap();
        let tx = base_tx(TransactionPayload::Script(Script::new(
            code,
            vec![TypeTag::Struct(Box::new(StructTag::aptos_coin()))],
            vec![TransactionArgument::U8(2)],
        )));
        assert_eq!(hex::encode(sign_and_build(&tx)), SIGNED_SCRIPT_VECTOR);
    }

    #[test]
    fn test_script_with_u16_u32_u256_args_matches_official_vector() {
        let mut u256_le = [0x11u8; 32];
        u256_le[31] = 0xf1;
        let tx = base_tx(TransactionPayload::Script(Script::new(
            vec![],
            vec![],
            vec![
                TransactionArgument::U16(0xf111),
                TransactionArgument::U32(0xf111_1111),
                TransactionArgument::U256(u256_le),
            ],
        )));
        assert_eq!(
            hex::encode(sign_and_build(&tx)),
            SIGNED_SCRIPT_NEW_INTS_VECTOR
        );
    }

    #[test]
    fn test_signature_round_trip_verifies_with_ed25519_dalek() {
        let tx = base_tx(TransactionPayload::EntryFunction(EntryFunction::new(
            ModuleId::new(
                AccountAddress::from_hex("0x1222").unwrap(),
                Identifier::new("aptos_coin").unwrap(),
            ),
            Identifier::new("transfer").unwrap(),
            vec![],
            transfer_args(),
        )));
        let key = signing_key();
        let message = tx.build_for_signing();
        let signature = key.sign(&message);
        key.verifying_key().verify(&message, &signature).unwrap();

        let signed = tx.build_with_signature(
            &Ed25519PublicKey::new(key.verifying_key().to_bytes()),
            &Ed25519Signature::new(signature.to_bytes()),
        );
        // SignedTransaction = raw BCS || 99-byte Ed25519 authenticator.
        let raw = tx.to_bcs_bytes();
        assert_eq!(signed.len(), raw.len() + 99);
        assert_eq!(&signed[..raw.len()], raw.as_slice());
        assert_eq!(signed[raw.len()], 0x00);
        assert_eq!(signed[raw.len() + 1], 0x20);
        assert_eq!(signed[raw.len() + 34], 0x40);
    }

    #[test]
    fn test_raw_transaction_bcs_matches_reference_bcs_tuple_model() {
        // Independent cross-check with the reference bcs crate: a
        // RawTransaction is structurally a tuple of its fields, with the
        // EntryFunction variant modeled as its ULEB128 index byte (2 < 128)
        // followed by the variant fields.
        let tx = base_tx(TransactionPayload::EntryFunction(EntryFunction::new(
            ModuleId::new(
                AccountAddress::from_hex("0x1222").unwrap(),
                Identifier::new("aptos_coin").unwrap(),
            ),
            Identifier::new("transfer").unwrap(),
            vec![],
            transfer_args(),
        )));
        let reference = bcs::to_bytes(&(
            tx.sender.into_inner(),
            0u64, // sequence_number
            2u8,  // TransactionPayload::EntryFunction index
            AccountAddress::from_hex("0x1222").unwrap().into_inner(),
            "aptos_coin",     // module name
            "transfer",       // function name
            Vec::<u8>::new(), // empty ty_args (empty seq = 0x00)
            transfer_args(),  // args as Vec<Vec<u8>>
            2000u64,          // max_gas_amount
            0u64,             // gas_unit_price
            u64::MAX,         // expiration_timestamp_secs
            4u8,              // chain_id
        ))
        .unwrap();
        assert_eq!(tx.to_bcs_bytes(), reference);
    }

    #[test]
    #[cfg(feature = "serde_json")]
    fn test_from_json_reproduces_official_vector() {
        let json = r#"{
            "sender": "0xa550c18",
            "sequence_number": "0",
            "payload": {
                "EntryFunction": {
                    "module": { "address": "0x1222", "name": "aptos_coin" },
                    "function": "transfer",
                    "ty_args": [],
                    "args": [
                        [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,221],
                        [1,0,0,0,0,0,0,0]
                    ]
                }
            },
            "max_gas_amount": 2000,
            "gas_unit_price": 0,
            "expiration_timestamp_secs": 18446744073709551615,
            "chain_id": 4
        }"#;
        let tx = AptosTransaction::from_json(json).unwrap();
        assert_eq!(hex::encode(tx.build_for_signing()), SIGNING_MESSAGE_VECTOR);
    }

    /// Every `u64` field must leave as a decimal string, and a `u64::MAX`
    /// expiry must come back bit-for-bit (a bare JSON number would be
    /// mangled to 18446744073709552000 by a JavaScript client).
    #[test]
    #[cfg(feature = "serde_json")]
    fn test_json_serializes_u64_fields_as_decimal_strings() {
        let tx = base_tx(TransactionPayload::EntryFunction(EntryFunction::new(
            ModuleId::new(
                AccountAddress::from_hex("0x1222").unwrap(),
                Identifier::new("aptos_coin").unwrap(),
            ),
            Identifier::new("transfer").unwrap(),
            vec![],
            transfer_args(),
        )));
        assert_eq!(tx.expiration_timestamp_secs, u64::MAX);

        let json = serde_json::to_string(&tx).unwrap();
        assert!(
            json.contains(r#""expiration_timestamp_secs":"18446744073709551615""#),
            "u64 must be quoted, got {json}"
        );
        assert!(json.contains(r#""sequence_number":"0""#), "{json}");
        assert!(json.contains(r#""max_gas_amount":"2000""#), "{json}");
        assert!(json.contains(r#""gas_unit_price":"0""#), "{json}");
        // No bare number above 2^53 anywhere in the document.
        assert!(!json.contains("18446744073709551615,"), "{json}");

        let back = AptosTransaction::from_json(&json).unwrap();
        assert_eq!(back.expiration_timestamp_secs, u64::MAX);
        assert_eq!(back, tx);
        assert_eq!(back.build_for_signing(), tx.build_for_signing());
    }

    /// Both JSON encodings of a `u64` must parse to the same transaction, so
    /// existing clients that send numbers keep working.
    #[test]
    #[cfg(feature = "serde_json")]
    fn test_from_json_accepts_numbers_and_strings_alike() {
        fn json_with(sequence_number: &str, expiration: &str) -> String {
            format!(
                r#"{{
                    "sender": "0xa550c18",
                    "sequence_number": {sequence_number},
                    "payload": {{
                        "EntryFunction": {{
                            "module": {{ "address": "0x1222", "name": "aptos_coin" }},
                            "function": "transfer",
                            "ty_args": [],
                            "args": [
                                [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,221],
                                [1,0,0,0,0,0,0,0]
                            ]
                        }}
                    }},
                    "max_gas_amount": 2000,
                    "gas_unit_price": 0,
                    "expiration_timestamp_secs": {expiration},
                    "chain_id": 4
                }}"#
            )
        }

        let from_numbers = AptosTransaction::from_json(&json_with("0", "18446744073709551615"))
            .expect("numbers must still parse");
        let from_strings =
            AptosTransaction::from_json(&json_with(r#""0""#, r#""18446744073709551615""#))
                .expect("strings must still parse");
        assert_eq!(from_numbers, from_strings);
        assert_eq!(
            hex::encode(from_numbers.build_for_signing()),
            SIGNING_MESSAGE_VECTOR
        );
        assert_eq!(
            hex::encode(from_strings.build_for_signing()),
            SIGNING_MESSAGE_VECTOR
        );
    }

    #[test]
    #[cfg(feature = "serde_json")]
    fn test_serde_json_round_trip() {
        let tx = base_tx(TransactionPayload::EntryFunction(EntryFunction::new(
            ModuleId::new(
                AccountAddress::from_hex("0x1222").unwrap(),
                Identifier::new("aptos_coin").unwrap(),
            ),
            Identifier::new("transfer").unwrap(),
            vec![TypeTag::Struct(Box::new(StructTag::aptos_coin()))],
            transfer_args(),
        )));
        let json = serde_json::to_string(&tx).unwrap();
        let back = AptosTransaction::from_json(&json).unwrap();
        assert_eq!(back, tx);
        assert_eq!(back.build_for_signing(), tx.build_for_signing());
    }
}
