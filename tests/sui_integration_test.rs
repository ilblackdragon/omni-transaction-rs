#![cfg(feature = "sui")]
//! End-to-end Sui tests: build a transaction with the omni builder, sign its
//! digest with a real ed25519 key and verify every byte against the
//! reference `sui-sdk-types` implementation. No network access.
use base64::{engine::general_purpose::STANDARD, Engine};
use ed25519_dalek::{Signer, SigningKey, Verifier};
use sui_sdk_types as sst;

use omni_transaction::sui::types::{
    Argument, CallArg, Command, ObjectDigest, ObjectRef, SignatureScheme, SuiSignature,
    TransactionExpiration,
};
use omni_transaction::sui::utils::derive_sui_address;
use omni_transaction::sui::SuiTransaction;
use omni_transaction::{TransactionBuilder, TxBuilder, SUI};

/// Deterministic dev-only signing key (never use a fixed seed in production).
const TEST_SEED: [u8; 32] = [0x42u8; 32];

const GAS_PRICE: u64 = 1000;
const GAS_BUDGET: u64 = 5_000_000;

fn build_transfer(signing_key: &SigningKey) -> SuiTransaction {
    // The sender must be the address derived from the signing key.
    let sender = derive_sui_address(
        SignatureScheme::Ed25519,
        &signing_key.verifying_key().to_bytes(),
    );
    let recipient = omni_transaction::sui::utils::parse_sui_address("0x3");

    TransactionBuilder::new::<SUI>()
        .sender(sender)
        .programmable(
            vec![
                CallArg::pure_u64(1_000_000),
                CallArg::pure_address(recipient),
            ],
            vec![
                Command::SplitCoins {
                    coin: Argument::GasCoin,
                    amounts: vec![Argument::Input(0)],
                },
                Command::TransferObjects {
                    objects: vec![Argument::Result(0)],
                    address: Argument::Input(1),
                },
            ],
        )
        .gas_payment(vec![ObjectRef::new(
            omni_transaction::sui::utils::parse_sui_address("0x1"),
            2,
            ObjectDigest::new([0x63u8; 32]),
        )])
        .gas_price(GAS_PRICE)
        .gas_budget(GAS_BUDGET)
        .expiration(TransactionExpiration::Epoch(500))
        .build()
}

fn build_reference_transfer(signing_key: &SigningKey) -> sst::Transaction {
    let sender =
        sst::Ed25519PublicKey::new(signing_key.verifying_key().to_bytes()).derive_address();

    sst::Transaction {
        kind: sst::TransactionKind::ProgrammableTransaction(sst::ProgrammableTransaction {
            inputs: vec![
                sst::Input::Pure(1_000_000u64.to_le_bytes().to_vec()),
                sst::Input::Pure(sst::Address::from_hex("0x3").unwrap().into_inner().to_vec()),
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
        sender,
        gas_payment: sst::GasPayment {
            objects: vec![sst::ObjectReference::new(
                sst::Address::from_hex("0x1").unwrap(),
                2,
                sst::Digest::new([0x63u8; 32]),
            )],
            owner: sender,
            price: GAS_PRICE,
            budget: GAS_BUDGET,
        },
        expiration: sst::TransactionExpiration::Epoch(500),
    }
}

#[test]
fn test_unsigned_tx_bytes_and_digest_match_reference() {
    let signing_key = SigningKey::from_bytes(&TEST_SEED);
    let omni_tx = build_transfer(&signing_key);
    let reference = build_reference_transfer(&signing_key);

    // Unsigned BCS bytes are identical.
    let tx_bytes = omni_tx.tx_bytes();
    let reference_bytes = bcs::to_bytes(&reference).expect("reference must serialize");
    assert_eq!(tx_bytes, reference_bytes);

    // The signing digest (blake2b-256 of the intent message) is identical.
    assert_eq!(
        omni_tx.build_for_signing(),
        reference.signing_digest().to_vec()
    );

    // The intent message is the digest preimage.
    let intent_message = omni_tx.build_intent_message();
    assert_eq!(&intent_message[..3], &[0u8, 0u8, 0u8]);
    assert_eq!(&intent_message[3..], tx_bytes.as_slice());
}

#[test]
fn test_end_to_end_sign_and_envelope_matches_reference() {
    let signing_key = SigningKey::from_bytes(&TEST_SEED);
    let verifying_key = signing_key.verifying_key();

    // 1. Build and produce the signing payload (final 32-byte digest).
    let omni_tx = build_transfer(&signing_key);
    let digest = omni_tx.build_for_signing();
    assert_eq!(digest.len(), 32);

    // 2. Sign the digest bytes verbatim (what a NEAR MPC ed25519 signer
    //    would do) and sanity-check the signature verifies over the digest.
    let signature = signing_key.sign(&digest);
    verifying_key
        .verify(&digest, &signature)
        .expect("signature must verify over the digest");

    // 3. Assemble the Sui signature and the canonical signed envelope.
    let sui_signature = SuiSignature::ed25519(signature.to_bytes(), verifying_key.to_bytes());
    let signed = omni_tx.build_with_signature(&sui_signature);

    // 4. Reference envelope: bcs(SenderSignedData) = seq(1) || intent(0,0,0)
    //    || bcs(TransactionData) || bcs(Vec<UserSignature>), per
    //    sui-sdk-types SignedTransactionWithIntentMessage.
    let reference = build_reference_transfer(&signing_key);
    let reference_signature = sst::UserSignature::Simple(sst::SimpleSignature::Ed25519 {
        signature: sst::Ed25519Signature::new(signature.to_bytes()),
        public_key: sst::Ed25519PublicKey::new(verifying_key.to_bytes()),
    });
    let mut expected = vec![0x01u8, 0x00, 0x00, 0x00];
    expected.extend_from_slice(&bcs::to_bytes(&reference).expect("reference must serialize"));
    expected.extend_from_slice(
        &bcs::to_bytes(&vec![reference_signature.clone()]).expect("signatures must serialize"),
    );
    assert_eq!(signed, expected);

    // 5. The reference SDK parses our raw signature bytes back.
    let parsed = sst::UserSignature::from_bytes(&sui_signature.to_bytes())
        .expect("reference must parse our signature");
    assert_eq!(parsed, reference_signature);
}

#[test]
fn test_json_rpc_payloads_match_reference_base64() {
    let signing_key = SigningKey::from_bytes(&TEST_SEED);
    let omni_tx = build_transfer(&signing_key);
    let reference = build_reference_transfer(&signing_key);

    // tx_bytes parameter: base64 of the unsigned BCS bytes.
    let tx_bytes_base64 = STANDARD.encode(omni_tx.tx_bytes());
    assert_eq!(
        tx_bytes_base64,
        STANDARD.encode(bcs::to_bytes(&reference).unwrap())
    );

    // signatures parameter: base64(flag || sig || pk), no length prefix.
    let digest = omni_tx.build_for_signing();
    let signature = signing_key.sign(&digest);
    let sui_signature =
        SuiSignature::ed25519(signature.to_bytes(), signing_key.verifying_key().to_bytes());
    let reference_signature = sst::UserSignature::Simple(sst::SimpleSignature::Ed25519 {
        signature: sst::Ed25519Signature::new(signature.to_bytes()),
        public_key: sst::Ed25519PublicKey::new(signing_key.verifying_key().to_bytes()),
    });
    assert_eq!(sui_signature.to_base64(), reference_signature.to_base64());

    // The transaction digest exposed for explorers is Sui's canonical
    // TransactionDigest (blake2b256("TransactionData::" || bcs)) — compared
    // against the reference SDK's independent digest computation.
    assert_eq!(omni_tx.digest_base58(), reference.digest().to_string());
}

/// A Sui transaction must reference at least one `Coin<SUI>` gas object.
/// Forgetting the gas payment must fail while building, not after an MPC
/// signature has been paid for and a validator rejects the transaction.
#[test]
#[should_panic(expected = "gas_payment is mandatory")]
fn test_build_without_gas_payment_panics() {
    let signing_key = SigningKey::from_bytes(&[0x11u8; 32]);
    let sender = derive_sui_address(
        SignatureScheme::Ed25519,
        &signing_key.verifying_key().to_bytes(),
    );
    let _ = TransactionBuilder::new::<SUI>()
        .sender(sender)
        .programmable(vec![], vec![])
        .gas_price(GAS_PRICE)
        .gas_budget(GAS_BUDGET)
        .build();
}
