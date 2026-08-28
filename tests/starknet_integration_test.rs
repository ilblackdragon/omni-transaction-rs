#![cfg(feature = "starknet")]
//! End-to-end tests for the Starknet INVOKE v3 module:
//! builder -> `build_for_signing` -> sign with reference crates -> `build_with_signature`,
//! verified byte-for-byte against real network transactions and independent
//! reference implementations (`starknet-crypto`, `starknet-core`, `ed25519-dalek`).

use ed25519_dalek::{Signer, SigningKey, Verifier};

use omni_transaction::starknet::types::{Call, DataAvailabilityMode, Felt, ResourceBounds};
use omni_transaction::starknet::utils::{
    cairo_short_string_to_felt, encode_calls, get_selector_from_name, CHAIN_ID_MAINNET,
    CHAIN_ID_SEPOLIA,
};
use omni_transaction::starknet::StarknetTransaction;
use omni_transaction::{TransactionBuilder, TxBuilder, STARKNET};

/// Real Sepolia invoke v3 transaction (block 567941, ACCEPTED_ON_L1):
/// 0x76b52e17bc09064bd986ead34263e6305ef3cecfb3ae9e19b86bf4f1a1a20ea.
const SEPOLIA_TX_HASH_HEX: &str =
    "076b52e17bc09064bd986ead34263e6305ef3cecfb3ae9e19b86bf4f1a1a20ea";

const SEPOLIA_SENDER: &str = "0x745d525a3582e91299d8d7c71730ffc4b1f191f5b219d800334bc0edad0983b";
const SEPOLIA_CALL_TO: &str = "0x4138fd51f90d171df37e9d4419c8cdb67d525840c58f8a5c347be93a1c5277d";
const SEPOLIA_CALL_SELECTOR: &str =
    "0x2468d193cd15b621b24c2a602b8dbcfa5eaa14f88416c40c09d7fd12592cb4b";

fn sepolia_calls() -> Vec<Call> {
    vec![Call {
        to: Felt::from_hex(SEPOLIA_CALL_TO).unwrap(),
        selector: Felt::from_hex(SEPOLIA_CALL_SELECTOR).unwrap(),
        calldata: vec![],
    }]
}

fn sepolia_transaction() -> StarknetTransaction {
    TransactionBuilder::new::<STARKNET>()
        .chain_id(CHAIN_ID_SEPOLIA)
        .sender_address(Felt::from_hex(SEPOLIA_SENDER).unwrap())
        .nonce(Felt::from_hex("0x9803").unwrap())
        .calls(&sepolia_calls())
        .tip(0)
        .l1_gas(ResourceBounds::new(0x186a0, 0x2d79883d20000))
        .l2_gas(ResourceBounds::new(0x5f5e100, 0xba43b7400))
        .l1_data_gas(ResourceBounds::new(0x186a0, 0x2d79883d20000))
        .build()
}

/// Independent SNIP-8 invoke v3 hash implementation, written directly against
/// `starknet-crypto` / `starknet-core` (the reference crates), mirroring
/// starkware-libs/sequencer `get_invoke_transaction_v3_hash`.
fn reference_invoke_v3_hash(tx: &StarknetTransaction) -> Felt {
    fn resource_felt(name: &[u8], bounds: &ResourceBounds) -> Felt {
        let mut buffer = [0u8; 32];
        buffer[8 - name.len()..8].copy_from_slice(name);
        buffer[8..16].copy_from_slice(&bounds.max_amount.to_be_bytes());
        buffer[16..32].copy_from_slice(&bounds.max_price_per_unit.to_be_bytes());
        Felt::from_bytes_be(&buffer)
    }

    let da_mode = |mode: DataAvailabilityMode| match mode {
        DataAvailabilityMode::L1 => 0u64,
        DataAvailabilityMode::L2 => 1u64,
    };

    let fee_fields_hash = starknet_crypto::poseidon_hash_many(&[
        Felt::from(tx.tip),
        resource_felt(b"L1_GAS", &tx.l1_gas),
        resource_felt(b"L2_GAS", &tx.l2_gas),
        resource_felt(b"L1_DATA", &tx.l1_data_gas),
    ]);

    starknet_crypto::poseidon_hash_many(&[
        starknet_core::utils::cairo_short_string_to_felt("invoke").unwrap(),
        Felt::THREE,
        tx.sender_address,
        fee_fields_hash,
        starknet_crypto::poseidon_hash_many(&tx.paymaster_data),
        tx.chain_id,
        tx.nonce,
        Felt::from(
            (da_mode(tx.nonce_data_availability_mode) << 32)
                + da_mode(tx.fee_data_availability_mode),
        ),
        starknet_crypto::poseidon_hash_many(&tx.account_deployment_data),
        starknet_crypto::poseidon_hash_many(&tx.calldata),
    ])
}

#[test]
fn test_build_for_signing_matches_real_sepolia_transaction_byte_for_byte() {
    let tx = sepolia_transaction();
    let payload = tx.build_for_signing();

    assert_eq!(payload.len(), 32);
    assert_eq!(hex::encode(&payload), SEPOLIA_TX_HASH_HEX);
}

#[test]
fn test_build_for_signing_matches_reference_hash_implementation() {
    // Real network vector.
    let tx = sepolia_transaction();
    assert_eq!(
        tx.build_for_signing(),
        reference_invoke_v3_hash(&tx).to_bytes_be().to_vec()
    );

    // Synthetic transaction exercising every non-default field.
    let synthetic = StarknetTransaction {
        chain_id: cairo_short_string_to_felt("PRIVATE_SN_POTC_SEPOLIA"),
        sender_address: Felt::from_hex("0xabcdef1234567890").unwrap(),
        nonce: Felt::from_hex("0xffffffffffffffffffffffffffffffff").unwrap(),
        calldata: encode_calls(&[
            Call {
                to: Felt::from(0x1111u64),
                selector: get_selector_from_name("transfer"),
                calldata: vec![Felt::from(7u64), Felt::from(8u64), Felt::from(9u64)],
            },
            Call {
                to: Felt::from(0x2222u64),
                selector: get_selector_from_name("approve"),
                calldata: vec![],
            },
        ]),
        tip: 0x1234,
        l1_gas: ResourceBounds::new(u64::MAX, u128::MAX),
        l2_gas: ResourceBounds::new(1, 2),
        l1_data_gas: ResourceBounds::new(3, 4),
        paymaster_data: vec![Felt::from(0xaaaau64)],
        account_deployment_data: vec![Felt::from(0xbbbbu64), Felt::from(0xccccu64)],
        nonce_data_availability_mode: DataAvailabilityMode::L2,
        fee_data_availability_mode: DataAvailabilityMode::L1,
    };

    assert_eq!(
        synthetic.build_for_signing(),
        reference_invoke_v3_hash(&synthetic).to_bytes_be().to_vec()
    );
}

#[test]
fn test_constants_and_selectors_match_starknet_core() {
    assert_eq!(CHAIN_ID_MAINNET, starknet_core::chain_id::MAINNET);
    assert_eq!(CHAIN_ID_SEPOLIA, starknet_core::chain_id::SEPOLIA);

    for name in ["__execute__", "__validate__", "transfer", "approve"] {
        assert_eq!(
            get_selector_from_name(name),
            starknet_core::utils::get_selector_from_name(name).unwrap(),
            "selector mismatch for {name:?}"
        );
    }
}

/// End to end with a Stark-curve account signature: builder -> build_for_signing ->
/// deterministic RFC 6979 ECDSA sign with `starknet-crypto` -> verify with
/// `starknet-crypto` -> build_with_signature -> byte-for-byte JSON check.
#[cfg(feature = "serde_json")]
#[test]
fn test_end_to_end_stark_curve_signature() {
    let tx = sepolia_transaction();

    // 1. The payload to sign is the 32-byte big-endian transaction-hash felt.
    let payload = tx.build_for_signing();
    let payload_bytes: [u8; 32] = payload.try_into().unwrap();
    let tx_hash = Felt::from_bytes_be(&payload_bytes);
    assert_eq!(tx_hash, Felt::from_hex(SEPOLIA_TX_HASH_HEX).unwrap());

    // 2. Sign the hash with the reference Stark-curve implementation (deterministic k).
    let private_key = Felt::from_hex("0x123456789abcdef").unwrap();
    let k = starknet_crypto::rfc6979_generate_k(&tx_hash, &private_key, None);
    let signature = starknet_crypto::sign(&private_key, &tx_hash, &k).unwrap();

    // 3. The reference implementation accepts the signature over exactly this payload.
    let public_key = starknet_crypto::get_public_key(&private_key);
    assert!(starknet_crypto::verify(&public_key, &tx_hash, &signature.r, &signature.s).unwrap());

    // 4. Assemble the broadcastable JSON with the Stark-curve [r, s] felt layout.
    let encoded = tx.build_with_signature(&[signature.r, signature.s]);
    let value: serde_json::Value = serde_json::from_slice(&encoded).unwrap();

    assert_eq!(value["type"], "INVOKE");
    assert_eq!(value["version"], "0x3");
    let sig_felts: Vec<Felt> = value["signature"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| Felt::from_hex(v.as_str().unwrap()).unwrap())
        .collect();
    assert_eq!(sig_felts, vec![signature.r, signature.s]);

    // 5. The signature the JSON carries still verifies over the payload we signed.
    assert!(starknet_crypto::verify(&public_key, &tx_hash, &sig_felts[0], &sig_felts[1]).unwrap());
}

/// The broadcast JSON with the REAL on-chain signature must match the
/// `BROADCASTED_INVOKE_TXN_V3` object of starknet-specs v0.9.0 byte-for-byte.
#[cfg(feature = "serde_json")]
#[test]
fn test_build_with_signature_matches_rpc_spec_byte_for_byte() {
    let tx = sepolia_transaction();

    // On-chain signature of the real Sepolia transaction.
    let signature = vec![
        Felt::from_hex("0x17bacc700df6c82682139e8e550078a5daa75dfe356577f78f7e57fd7c56245")
            .unwrap(),
        Felt::from_hex("0x4eb8734727eb9412b79ba6d14ff1c9a6beb0dc0b811e3f97168c747f8d427b3")
            .unwrap(),
    ];

    let encoded = tx.build_with_signature(&signature);

    let expected = serde_json::json!({
        "type": "INVOKE",
        "version": "0x3",
        "sender_address": SEPOLIA_SENDER,
        "calldata": ["0x1", SEPOLIA_CALL_TO, SEPOLIA_CALL_SELECTOR, "0x0"],
        "signature": [
            "0x17bacc700df6c82682139e8e550078a5daa75dfe356577f78f7e57fd7c56245",
            "0x4eb8734727eb9412b79ba6d14ff1c9a6beb0dc0b811e3f97168c747f8d427b3"
        ],
        "nonce": "0x9803",
        "resource_bounds": {
            "l1_gas": { "max_amount": "0x186a0", "max_price_per_unit": "0x2d79883d20000" },
            "l1_data_gas": { "max_amount": "0x186a0", "max_price_per_unit": "0x2d79883d20000" },
            "l2_gas": { "max_amount": "0x5f5e100", "max_price_per_unit": "0xba43b7400" }
        },
        "tip": "0x0",
        "paymaster_data": [],
        "account_deployment_data": [],
        "nonce_data_availability_mode": "L1",
        "fee_data_availability_mode": "L1"
    });

    // Byte-for-byte: both sides serialize through serde_json::Value.
    assert_eq!(encoded, serde_json::to_vec(&expected).unwrap());

    // And the RPC params wrapper carries the same body under "invoke_transaction".
    let params: serde_json::Value =
        serde_json::from_slice(&tx.to_rpc_params_json(&signature)).unwrap();
    assert_eq!(params["invoke_transaction"], expected);
}

/// End to end with an ed25519 account signature (the NEAR MPC ed25519 path):
/// the payload is signed as an arbitrary 32-byte message, converted to felts,
/// round-tripped through the broadcast JSON, and verified again with `ed25519-dalek`.
#[cfg(feature = "serde_json")]
#[test]
fn test_end_to_end_ed25519_signature_round_trip() {
    let tx = sepolia_transaction();
    let payload = tx.build_for_signing();

    // Deterministic reference keypair.
    let signing_key = SigningKey::from_bytes(&[42u8; 32]);
    let signature = signing_key.sign(&payload);
    signing_key
        .verifying_key()
        .verify(&payload, &signature)
        .unwrap();

    // Convert the 64-byte signature into an account-defined felt layout:
    // four 16-byte big-endian limbs (each < 2^128, so always a valid felt).
    let sig_bytes = signature.to_bytes();
    let sig_felts: Vec<Felt> = sig_bytes
        .chunks(16)
        .map(Felt::from_bytes_be_slice)
        .collect();
    assert_eq!(sig_felts.len(), 4);

    // Round-trip through the broadcast JSON.
    let encoded = tx.build_with_signature(&sig_felts);
    let value: serde_json::Value = serde_json::from_slice(&encoded).unwrap();
    let recovered: Vec<u8> = value["signature"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|v| Felt::from_hex(v.as_str().unwrap()).unwrap().to_bytes_be()[16..].to_vec())
        .collect();

    let recovered_signature = ed25519_dalek::Signature::from_bytes(&recovered.try_into().unwrap());
    assert_eq!(recovered_signature, signature);
    signing_key
        .verifying_key()
        .verify(&payload, &recovered_signature)
        .unwrap();
}

/// Historical mainnet invoke v3 (block 636864, pre-0.13.4 two-resource fee hash),
/// recomputed with the reference crates to pin the resource-felt packing:
/// 0x1d4735f4ba73a67be2f648d9b21cab3783383b8c229566b46b027c46012219.
#[test]
fn test_historical_mainnet_vector_with_reference_crates() {
    fn resource_felt(name: &[u8], max_amount: u64, max_price_per_unit: u128) -> Felt {
        let mut buffer = [0u8; 32];
        buffer[8 - name.len()..8].copy_from_slice(name);
        buffer[8..16].copy_from_slice(&max_amount.to_be_bytes());
        buffer[16..32].copy_from_slice(&max_price_per_unit.to_be_bytes());
        Felt::from_bytes_be(&buffer)
    }

    let calldata = encode_calls(&[Call {
        to: Felt::from_hex("0x4c0a5193d58f74fbace4b74dcf65481e734ed1714121bdc571da345540efa05")
            .unwrap(),
        selector: Felt::from_hex(
            "0x3943907ef0ef6f9d2e2408b05e520a66daaf74293dbf665e5a20b117676170e",
        )
        .unwrap(),
        calldata: vec![
            Felt::from_hex("0x49d36570d4e46f48e99674bd3fcc84644ddd6b96f7c741b1562b82f9e004dc7")
                .unwrap(),
            Felt::from_hex("0x16345785d8a0000").unwrap(),
        ],
    }]);

    // Pre-0.13.4 fee hash: tip, L1_GAS, L2_GAS — no L1_DATA felt.
    let fee_fields_hash = starknet_crypto::poseidon_hash_many(&[
        Felt::ZERO,
        resource_felt(b"L1_GAS", 0xa9e, 0x7f2a1ad4f2f1),
        resource_felt(b"L2_GAS", 0, 0),
    ]);

    let hash = starknet_crypto::poseidon_hash_many(&[
        cairo_short_string_to_felt("invoke"),
        Felt::THREE,
        Felt::from_hex("0x69c0f9bcd79697bdceaf7748e3ff8f34aa39e4063ce44896af664c0c96f6c10")
            .unwrap(),
        fee_fields_hash,
        starknet_crypto::poseidon_hash_many(&[]),
        CHAIN_ID_MAINNET,
        Felt::from_hex("0x9d").unwrap(),
        Felt::ZERO,
        starknet_crypto::poseidon_hash_many(&[]),
        starknet_crypto::poseidon_hash_many(&calldata),
    ]);

    assert_eq!(
        hash,
        Felt::from_hex("0x1d4735f4ba73a67be2f648d9b21cab3783383b8c229566b46b027c46012219").unwrap()
    );
}
