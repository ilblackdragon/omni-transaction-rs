#![cfg(feature = "aptos")]
//! End-to-end tests for the Aptos module: builder -> build_for_signing ->
//! sign with the reference ed25519-dalek crate -> build_with_signature ->
//! byte-for-byte comparison against the official Aptos TS SDK vectors
//! (aptos-core @ aptos-node-v1.5.0, `transaction_builder.test.ts`) and
//! against the reference bcs crate.
use ed25519_dalek::{Signer, SigningKey, Verifier};
use omni_transaction::aptos::types::{
    AccountAddress, Ed25519PublicKey, Ed25519Signature, EntryFunction, Identifier, ModuleId,
    Script, StructTag, TransactionArgument, TransactionPayload, TypeTag,
};
use omni_transaction::aptos::AptosTransaction;
use omni_transaction::{TransactionBuilder, TxBuilder, APTOS};

/// Official TS SDK test seed; ed25519 signing is deterministic, so the
/// published signatures must be reproduced exactly.
const SEED_HEX: &str = "9bf49a6a0755f953811fce125f2683d50429c3bb49e074147e0089a52eae155f";
const PUBLIC_KEY_HEX: &str = "b9c6ee1630ef3e711144a648db06bbb2284f7274cfbee53ffcee503cc1a49200";

/// Signing message vector (prefix || BCS) for
/// 0x1222::aptos_coin::transfer(addr 0xdd, u64 1).
const SIGNING_MESSAGE_VECTOR: &str = "b5e97db07fa0bd0e5598aa3643a9bc6f6693bddc1a9fec9e674a461eaa00b193000000000000000000000000000000000000000000000000000000000a550c1800000000000000000200000000000000000000000000000000000000000000000000000000000012220a6170746f735f636f696e087472616e7366657200022000000000000000000000000000000000000000000000000000000000000000dd080100000000000000d0070000000000000000000000000000ffffffffffffffff04";

/// Full SignedTransaction (broadcast) vectors from the same test file.
const SIGNED_ENTRY_FUNCTION_VECTOR: &str = "000000000000000000000000000000000000000000000000000000000a550c1800000000000000000200000000000000000000000000000000000000000000000000000000000012220a6170746f735f636f696e087472616e7366657200022000000000000000000000000000000000000000000000000000000000000000dd080100000000000000d0070000000000000000000000000000ffffffffffffffff040020b9c6ee1630ef3e711144a648db06bbb2284f7274cfbee53ffcee503cc1a49200409c570996380897f38b8d7008d726fb45d6ded0689216e56b73f523492cba92deb6671c27e9a44d2a6fdfdb497420d00c621297a23d6d0298895e0d58cff6060c";
const SIGNED_ENTRY_FUNCTION_WITH_TY_ARGS_VECTOR: &str = "000000000000000000000000000000000000000000000000000000000a550c18000000000000000002000000000000000000000000000000000000000000000000000000000000122204636f696e087472616e73666572010700000000000000000000000000000000000000000000000000000000000000010a6170746f735f636f696e094170746f73436f696e00022000000000000000000000000000000000000000000000000000000000000000dd080100000000000000d0070000000000000000000000000000ffffffffffffffff040020b9c6ee1630ef3e711144a648db06bbb2284f7274cfbee53ffcee503cc1a4920040112162f543ca92b4f14c1b09b7f52894a127f5428b0d407c09c8efb3a136cff50e550aea7da1226f02571d79230b80bd79096ea0d796789ad594b8fbde695404";
const SIGNED_SCRIPT_VECTOR: &str = "000000000000000000000000000000000000000000000000000000000a550c1800000000000000000026a11ceb0b030000000105000100000000050601000000000000000600000000000000001a0102010700000000000000000000000000000000000000000000000000000000000000010a6170746f735f636f696e094170746f73436f696e00010002d0070000000000000000000000000000ffffffffffffffff040020b9c6ee1630ef3e711144a648db06bbb2284f7274cfbee53ffcee503cc1a49200409936b8d22cec685e720761f6c6135e020911f1a26e220e2a0f3317f5a68942531987259ac9e8688158c77df3e7136637056047d9524edad88ee45d61a9346602";
const SIGNED_SCRIPT_NEW_INTS_VECTOR: &str = "000000000000000000000000000000000000000000000000000000000a550c180000000000000000000000030611f107111111f10811111111111111111111111111111111111111111111111111111111111111f1d0070000000000000000000000000000ffffffffffffffff040020b9c6ee1630ef3e711144a648db06bbb2284f7274cfbee53ffcee503cc1a49200409402b773f66cf5444efe4de38a026cf9b34e0327798ea01f0695db8e8e0888e20387b08f504b620dcffbc382e3ac141c0ec9a820c5f58b5da2eec589a9e86b0b";

fn signing_key() -> SigningKey {
    let mut seed = [0u8; 32];
    hex::decode_to_slice(SEED_HEX, &mut seed).unwrap();
    SigningKey::from_bytes(&seed)
}

fn build_tx(payload: TransactionPayload) -> AptosTransaction {
    TransactionBuilder::new::<APTOS>()
        .sender(AccountAddress::from_hex("0xa550c18").unwrap())
        .sequence_number(0)
        .payload(payload)
        .max_gas_amount(2000)
        .gas_unit_price(0)
        .expiration_timestamp_secs(u64::MAX)
        .chain_id(4)
        .build()
}

fn transfer_args() -> Vec<Vec<u8>> {
    // Each arg is the BCS of the value: 32 raw bytes for the address, 8
    // little-endian bytes for the u64 amount.
    let receiver = AccountAddress::from_hex("0xdd").unwrap();
    vec![receiver.as_bytes().to_vec(), 1u64.to_le_bytes().to_vec()]
}

fn aptos_coin_type_tag() -> TypeTag {
    TypeTag::Struct(Box::new(StructTag::new(
        AccountAddress::ONE,
        Identifier::new("aptos_coin").unwrap(),
        Identifier::new("AptosCoin").unwrap(),
        vec![],
    )))
}

/// Full pipeline: build -> sign (ed25519-dalek) -> assemble -> verify against
/// the official vector and re-verify the embedded signature.
fn sign_and_check(tx: &AptosTransaction, expected_hex: &str) {
    let key = signing_key();
    let verifying_key = key.verifying_key();
    assert_eq!(hex::encode(verifying_key.to_bytes()), PUBLIC_KEY_HEX);

    let signing_message = tx.build_for_signing();
    let dalek_signature = key.sign(&signing_message);
    // Aptos signs the (prefix || BCS) preimage directly; verify it as
    // arbitrary bytes like the chain does.
    verifying_key
        .verify(&signing_message, &dalek_signature)
        .unwrap();

    let public_key = Ed25519PublicKey::new(verifying_key.to_bytes());
    let signature = Ed25519Signature::new(dalek_signature.to_bytes());
    let broadcast_bytes = tx.build_with_signature(&public_key, &signature);
    assert_eq!(hex::encode(&broadcast_bytes), expected_hex);

    // The broadcast bytes must embed the raw txn unchanged, followed by the
    // 99-byte Ed25519 authenticator; the authenticator must byte-match the
    // reference bcs crate's encoding of (variant 0, key bytes, sig bytes).
    let raw = tx.to_bcs_bytes();
    assert_eq!(&broadcast_bytes[..raw.len()], raw.as_slice());
    let reference_authenticator = bcs::to_bytes(&(
        0u8,
        verifying_key.to_bytes().to_vec(),
        dalek_signature.to_bytes().to_vec(),
    ))
    .unwrap();
    assert_eq!(&broadcast_bytes[raw.len()..], reference_authenticator);
}

#[test]
fn test_entry_function_end_to_end_matches_official_vectors() {
    let tx = build_tx(TransactionPayload::EntryFunction(EntryFunction::new(
        ModuleId::new(
            AccountAddress::from_hex("0x1222").unwrap(),
            Identifier::new("aptos_coin").unwrap(),
        ),
        Identifier::new("transfer").unwrap(),
        vec![],
        transfer_args(),
    )));
    assert_eq!(hex::encode(tx.build_for_signing()), SIGNING_MESSAGE_VECTOR);
    sign_and_check(&tx, SIGNED_ENTRY_FUNCTION_VECTOR);
}

#[test]
fn test_entry_function_with_type_args_end_to_end_matches_official_vector() {
    let tx = build_tx(TransactionPayload::EntryFunction(EntryFunction::new(
        ModuleId::new(
            AccountAddress::from_hex("0x1222").unwrap(),
            Identifier::new("coin").unwrap(),
        ),
        Identifier::new("transfer").unwrap(),
        vec![aptos_coin_type_tag()],
        transfer_args(),
    )));
    sign_and_check(&tx, SIGNED_ENTRY_FUNCTION_WITH_TY_ARGS_VECTOR);
}

#[test]
fn test_script_with_type_arg_and_u8_arg_end_to_end_matches_official_vector() {
    let code =
        hex::decode("a11ceb0b030000000105000100000000050601000000000000000600000000000000001a0102")
            .unwrap();
    let tx = build_tx(TransactionPayload::Script(Script::new(
        code,
        vec![aptos_coin_type_tag()],
        vec![TransactionArgument::U8(2)],
    )));
    sign_and_check(&tx, SIGNED_SCRIPT_VECTOR);
}

#[test]
fn test_script_with_new_integer_types_end_to_end_matches_official_vector() {
    let mut u256_le = [0x11u8; 32];
    u256_le[31] = 0xf1;
    let tx = build_tx(TransactionPayload::Script(Script::new(
        vec![],
        vec![],
        vec![
            TransactionArgument::U16(0xf111),
            TransactionArgument::U32(0xf111_1111),
            TransactionArgument::U256(u256_le),
        ],
    )));
    sign_and_check(&tx, SIGNED_SCRIPT_NEW_INTS_VECTOR);
}

#[test]
fn test_raw_transaction_bcs_matches_reference_bcs_crate() {
    // Structural cross-check with the reference bcs crate: a RawTransaction
    // is a tuple of its fields; the EntryFunction enum variant is its
    // ULEB128 index byte (2 < 128 == one raw byte) followed by its fields.
    let tx = build_tx(TransactionPayload::EntryFunction(EntryFunction::new(
        ModuleId::new(
            AccountAddress::from_hex("0x1222").unwrap(),
            Identifier::new("aptos_coin").unwrap(),
        ),
        Identifier::new("transfer").unwrap(),
        vec![],
        transfer_args(),
    )));
    let reference = bcs::to_bytes(&(
        AccountAddress::from_hex("0xa550c18").unwrap().into_inner(),
        0u64,
        2u8, // TransactionPayload::EntryFunction variant index
        AccountAddress::from_hex("0x1222").unwrap().into_inner(),
        "aptos_coin",
        "transfer",
        Vec::<u8>::new(), // empty ty_args sequence
        transfer_args(),
        2000u64,
        0u64,
        u64::MAX,
        4u8,
    ))
    .unwrap();
    assert_eq!(tx.to_bcs_bytes(), reference);
    // And build_for_signing is exactly the 32-byte salt digest plus these bytes.
    assert_eq!(tx.build_for_signing()[32..], reference[..]);
}

#[test]
#[cfg(feature = "serde_json")]
fn test_from_json_end_to_end_matches_official_vector() {
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
    sign_and_check(&tx, SIGNED_ENTRY_FUNCTION_VECTOR);
}

/// A serialize -> JSON -> deserialize round trip must be lossless for a
/// `u64::MAX` expiration: the u64 fields go out as decimal strings, so a
/// JavaScript client's `JSON.parse`/`JSON.stringify` cannot round them to
/// 18446744073709552000 and change the signing message.
#[test]
#[cfg(feature = "serde_json")]
fn test_json_round_trip_preserves_u64_max_expiration() {
    let tx = TransactionBuilder::new::<APTOS>()
        .sender(AccountAddress::from_hex("0xa550c18").unwrap())
        .sequence_number(0)
        .payload(TransactionPayload::EntryFunction(EntryFunction::new(
            ModuleId::new(
                AccountAddress::from_hex("0x1222").unwrap(),
                Identifier::new("aptos_coin").unwrap(),
            ),
            Identifier::new("transfer").unwrap(),
            vec![],
            transfer_args(),
        )))
        .max_gas_amount(2000)
        .gas_unit_price(0)
        .expiration_timestamp_secs(u64::MAX)
        .chain_id(4)
        .build();

    let json = serde_json::to_string(&tx).unwrap();
    assert!(
        json.contains(r#""expiration_timestamp_secs":"18446744073709551615""#),
        "u64 fields must be emitted as decimal strings, got {json}"
    );

    let back = AptosTransaction::from_json(&json).unwrap();
    assert_eq!(back, tx);
    assert_eq!(
        hex::encode(back.build_for_signing()),
        SIGNING_MESSAGE_VECTOR
    );
    sign_and_check(&back, SIGNED_ENTRY_FUNCTION_VECTOR);
}
