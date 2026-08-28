#![cfg(feature = "ton")]
//! End-to-end TON tests: build transactions with the omni builder, sign the
//! 32-byte payload with a real ed25519 key, and verify every byte against
//! the independent `tonlib-core` implementation. No network access.
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier};
use tonlib_core::cell::{BagOfCells, CellBuilder as TlCellBuilder};
use tonlib_core::tlb_types::block::message::{CommonMsgInfo, Message};
use tonlib_core::tlb_types::tlb::TLB;
use tonlib_core::wallet::version_helper::VersionHelper;
use tonlib_core::wallet::versioned::v4::WalletDataV4;
use tonlib_core::wallet::versioned::v5::WalletDataV5;
use tonlib_core::wallet::wallet_version::WalletVersion as TlWalletVersion;
use tonlib_core::{TonAddress as TlAddress, TonHash};

use omni_transaction::ton::types::{
    parse_boc_single_root, serialize_boc, Cell, CellBuilder, Coins, InternalMessage, TonAddress,
    WalletVersion,
};
use omni_transaction::ton::utils::derive_wallet_address;
use omni_transaction::ton::TonTransaction;
use omni_transaction::{TransactionBuilder, TxBuilder, TON};

/// Deterministic dev-only signing seed (never use a fixed seed in
/// production); its public key is
/// `31debe55d37c722768b137131caa6087080b2e0b60b94bd785d14575cfa498bc`.
const TEST_SEED: [u8; 32] = [0x17; 32];
const VALID_UNTIL: u64 = 1_735_689_600;
const TRANSFER_DEST: &str = "0:83dfd552e63729b472fcbcc8c45ebcc6691702558b68ec7527e1ba403a0f31a8";
const TRANSFER_NANOTON: u128 = 50_000_000;

fn signing_key() -> SigningKey {
    let key = SigningKey::from_bytes(&TEST_SEED);
    assert_eq!(
        hex::encode(key.verifying_key().to_bytes()),
        "31debe55d37c722768b137131caa6087080b2e0b60b94bd785d14575cfa498bc"
    );
    key
}

fn public_key() -> [u8; 32] {
    signing_key().verifying_key().to_bytes()
}

fn transfer() -> InternalMessage {
    let dest: TonAddress = TRANSFER_DEST.parse().unwrap();
    InternalMessage::new(dest, Coins::from_nano(TRANSFER_NANOTON))
}

fn sign(tx: &TonTransaction, key: &SigningKey) -> [u8; 64] {
    let payload = tx.build_for_signing();
    assert_eq!(payload.len(), 32);
    key.sign(&payload).to_bytes()
}

/// Returns tonlib-core's representation hash of a cell as a byte array
/// (the inherent method, disambiguated from the `TLB` trait method of the
/// same name).
fn tl_hash(cell: &tonlib_core::cell::Cell) -> [u8; 32] {
    let mut out = [0u8; 32];
    out.copy_from_slice(tonlib_core::cell::Cell::cell_hash(cell).as_slice());
    out
}

/// Every BoC we emit parses in tonlib-core and hashes identically there.
#[test]
fn test_cell_hashes_match_tonlib() {
    let leaf = CellBuilder::new()
        .store_uint(0xF, 32)
        .unwrap()
        .build()
        .unwrap();
    let seven_bit = CellBuilder::new()
        .store_uint(5, 7)
        .unwrap()
        .build()
        .unwrap();
    let tree = CellBuilder::new()
        .store_uint(0xDEAD_BEEF, 32)
        .unwrap()
        .store_ref(leaf)
        .unwrap()
        .store_ref(seven_bit)
        .unwrap()
        .build()
        .unwrap();
    let message_cell = transfer().to_cell().unwrap();

    for cell in [Cell::empty(), tree, message_cell] {
        for with_crc in [false, true] {
            let boc = serialize_boc(&cell, with_crc);
            let parsed = BagOfCells::parse(&boc).unwrap().single_root().unwrap();
            assert_eq!(tl_hash(&parsed), cell.repr_hash());
            assert_eq!(parsed.bit_len(), usize::from(cell.bit_len()));
        }
    }
}

/// BoCs produced by tonlib-core parse in our implementation with identical
/// hashes (the reverse direction).
#[test]
fn test_parse_tonlib_output() {
    let mut inner_builder = TlCellBuilder::new();
    inner_builder.store_string("differential").unwrap();
    let inner = inner_builder.build().unwrap();
    let mut builder = TlCellBuilder::new();
    builder
        .store_u32(17, 12345)
        .unwrap()
        .store_bit(true)
        .unwrap()
        .store_child(inner)
        .unwrap();
    let tl_cell = builder.build().unwrap();
    let boc = BagOfCells::from_root(tl_cell.clone())
        .serialize(true)
        .unwrap();
    let ours = parse_boc_single_root(&boc).unwrap();
    assert_eq!(ours.repr_hash(), tl_hash(&tl_cell));
    assert_eq!(usize::from(ours.bit_len()), tl_cell.bit_len());
}

/// Wallet addresses derived by us equal tonlib-core's derivation from its
/// own embedded wallet code and initial-data layouts.
#[test]
fn test_wallet_addresses_match_tonlib() {
    let pk = public_key();
    let tl_pk = TonHash::try_from(pk.as_slice()).unwrap();

    let cases: &[(WalletVersion, TlWalletVersion, i8, u32)] = &[
        (WalletVersion::V4R2, TlWalletVersion::V4R2, 0, 698_983_191),
        (WalletVersion::V5R1, TlWalletVersion::V5R1, 0, 0x7FFF_FF11),
        // v5r1 testnet wallet id.
        (WalletVersion::V5R1, TlWalletVersion::V5R1, 0, 0x7FFF_FFFD),
    ];
    for &(version, tl_version, workchain, wallet_id) in cases {
        let ours = derive_wallet_address(version, workchain, wallet_id, &pk);

        let tl_data = match tl_version {
            TlWalletVersion::V4R2 => WalletDataV4::new(wallet_id as i32, tl_pk.clone())
                .to_cell()
                .unwrap(),
            _ => WalletDataV5::new(wallet_id as i32, tl_pk.clone())
                .to_cell()
                .unwrap(),
        };
        // Our initial data cell matches tonlib's byte-for-byte.
        assert_eq!(
            version.initial_data_cell(wallet_id, &pk).repr_hash(),
            tl_hash(&tl_data)
        );
        let tl_code = VersionHelper::get_code(tl_version).unwrap().clone();
        let tl_address =
            TlAddress::derive(i32::from(workchain), tl_code, tl_data.to_arc()).unwrap();
        assert_eq!(ours.to_string(), tl_address.to_base64_url());
        assert_eq!(ours.to_raw_string(), tl_address.to_hex());
    }
}

/// The embedded wallet code cells equal tonlib-core's embedded resources.
#[test]
fn test_embedded_wallet_code_matches_tonlib() {
    for (version, tl_version) in [
        (WalletVersion::V4R2, TlWalletVersion::V4R2),
        (WalletVersion::V5R1, TlWalletVersion::V5R1),
    ] {
        let tl_code = VersionHelper::get_code(tl_version).unwrap();
        assert_eq!(version.code_cell().repr_hash(), tl_hash(tl_code));
        assert_eq!(version.code_hash(), tl_hash(tl_code));
    }
}

/// Rebuilds a cell from `body` without its first `skip` and last `drop`
/// bits (keeping all references) and returns its hash — used to recompute
/// the unsigned-body hash from a parsed signed body.
fn tl_rebuild_hash(body: &tonlib_core::cell::Cell, skip: usize, drop: usize) -> [u8; 32] {
    let keep = body.bit_len() - skip - drop;
    let mut parser = body.parser();
    let _ = parser.load_bits(skip).unwrap();
    let bits = parser.load_bits(keep).unwrap();
    let mut builder = TlCellBuilder::new();
    builder.store_bits(keep, &bits).unwrap();
    for reference in body.references() {
        builder.store_reference(reference).unwrap();
    }
    tl_hash(&builder.build().unwrap())
}

/// Extracts `count` bits starting at `skip` from a parsed cell as bytes.
fn tl_extract_bits(body: &tonlib_core::cell::Cell, skip: usize, count: usize) -> Vec<u8> {
    let mut parser = body.parser();
    let _ = parser.load_bits(skip).unwrap();
    parser.load_bits(count).unwrap()
}

/// Full v4r2 round trip: build -> sign (ed25519-dalek) -> wrap -> the exact
/// spec vectors -> parse the broadcast bytes with tonlib-core and verify the
/// signature over the recomputed body hash.
#[test]
fn test_v4r2_end_to_end_round_trip() {
    let key = signing_key();
    let mut message = transfer();
    message.mode = 1;
    let tx = TransactionBuilder::new::<TON>()
        .wallet_version(WalletVersion::V4R2)
        .public_key(public_key())
        .seqno(1)
        .valid_until(VALID_UNTIL)
        .add_message(message)
        .build();

    // Signing payload and deterministic ed25519 signature match the spec.
    let payload = tx.build_for_signing();
    assert_eq!(
        hex::encode(&payload),
        "84d81f593253be4c06ab026b2e44e78fc5d9a98a634021fd68b9519d1e8f4b83"
    );
    let signature = sign(&tx, &key);
    assert_eq!(
        hex::encode(signature),
        "4afc182e0f399fbed7a305589e8ba57fb568135def13ead61fce81200d8cfa99f0e93af3c47974733a05ccf07075e954d9e23dcde46bb52885cb88dcb9840f09"
    );

    let boc = tx.build_with_signature(signature);
    assert_eq!(
        tx.build_with_signature_base64(signature),
        "te6cckEBAgEAqgAB4YgBdE5I68D65FHDm6hlGUdLtg5pgrhZz3sjCaVuA6Pr9MgCV+DBcHnM/fa9GCrE9F0r/atAmu94n1aw/nQJAGxn1M+HSdeeI8ujmdAuZ4ODr0qmzxHubyNdqUQuXEblzCB4SU1NGLs7pCwAAAAACAAMAQBoYgBB7+qpcxuU2jl+XmRiL15jNIuBKsW0djqT8N0gHQeY1CAX14QAAAAAAAAAAAAAAAAAAMjj5qM="
    );

    // Parse the broadcast bytes with tonlib-core.
    let parsed = Message::from_boc(&boc).unwrap();
    let CommonMsgInfo::ExtIn(info) = &parsed.info else {
        panic!("expected an external-in message");
    };
    let dest = TlAddress::from_msg_address(info.dest.clone()).unwrap();
    assert_eq!(tx.wallet_address().to_string(), dest.to_base64_url());
    assert!(parsed.init.is_none());

    // The body carries the signature in its FIRST 512 bits; the signed
    // payload is the hash of the body without them.
    let body = parsed.body.value.clone();
    assert_eq!(
        tl_hash(&body).to_vec(),
        hex::decode("4c3b549e6f23d609af6896d96c923ede11cf30bef8a288bb8e71f7a70533858f").unwrap()
    );
    let recomputed = tl_rebuild_hash(&body, 512, 0);
    assert_eq!(recomputed.to_vec(), payload);
    let extracted = tl_extract_bits(&body, 0, 512);
    key.verifying_key()
        .verify(&recomputed, &Signature::from_slice(&extracted).unwrap())
        .unwrap();
}

/// Full v5r1 round trip with the signature APPENDED, verified the same way.
#[test]
fn test_v5r1_end_to_end_round_trip() {
    let key = signing_key();
    let tx = TransactionBuilder::new::<TON>()
        .public_key(public_key())
        .seqno(1)
        .valid_until(VALID_UNTIL)
        .add_message(transfer())
        .build();
    assert_eq!(tx.wallet_version, WalletVersion::V5R1);

    let payload = tx.build_for_signing();
    assert_eq!(
        hex::encode(&payload),
        "0b45a74ddb8c4bd8f9a504b255869daad1bba81a1bd03351ea5ca1a3c47a2416"
    );
    let signature = sign(&tx, &key);
    let boc = tx.build_with_signature(signature);
    assert_eq!(
        tx.build_with_signature_base64(signature),
        "te6cckEBBAEAtwAB5YgAsZrrPTzadQXWGoydYFoYGfjndoWc/Wqh4SHr0/RYsl4Dm0s7c///+Is7pCwAAAAADWkaJkxhW/VO+P3HsjO8WONg2oHKbvGnROVSuwi4zGMomSwfBbyvGpxOAvB7/VSEDrrWDXSJ2QMoNn/6J31+uAsBAgoOw8htAwIDAAAAaGIAQe/qqXMblNo5fl5kYi9eYzSLgSrFtHY6k/DdIB0HmNQgF9eEAAAAAAAAAAAAAAAAAACThNw1"
    );

    let parsed = Message::from_boc(&boc).unwrap();
    let CommonMsgInfo::ExtIn(info) = &parsed.info else {
        panic!("expected an external-in message");
    };
    let dest = TlAddress::from_msg_address(info.dest.clone()).unwrap();
    assert_eq!(tx.wallet_address().to_string(), dest.to_base64_url());

    // The body carries the signature in its LAST 512 bits.
    let body = parsed.body.value.clone();
    assert_eq!(
        tl_hash(&body).to_vec(),
        hex::decode("bf8c025e5016fef348eb773c9fc8d44d0090fe53b52d6af9462dced7567fa955").unwrap()
    );
    let recomputed = tl_rebuild_hash(&body, 0, 512);
    assert_eq!(recomputed.to_vec(), payload);
    let extracted = tl_extract_bits(&body, body.bit_len() - 512, 512);
    key.verifying_key()
        .verify(&recomputed, &Signature::from_slice(&extracted).unwrap())
        .unwrap();
}

/// Deploy variant: the external message embeds the StateInit; tonlib-core
/// re-derives the same wallet address from the embedded code and data.
#[test]
fn test_v4r2_deploy_state_init_round_trip() {
    let key = signing_key();
    let mut message = transfer();
    message.mode = 1;
    let tx = TransactionBuilder::new::<TON>()
        .wallet_version(WalletVersion::V4R2)
        .public_key(public_key())
        .seqno(1)
        .valid_until(VALID_UNTIL)
        .messages(vec![message])
        .deploy(true)
        .build();

    let signature = sign(&tx, &key);
    let boc = tx.build_with_signature(signature);
    assert_eq!(
        tx.build_with_signature_base64(signature),
        "te6cckECFwEAA6wAA+OIAXROSOvA+uRRw5uoZRlHS7YOaYK4Wc97IwmlbgOj6/TIEYlfgwXB5zP32vRgqxPRdK/2rQJrveJ9WsP50CQBsZ9TPh0nXniPLo5nQLmeDg69Kps8R7m8jXalELlxG5cwgeElNTRi7O6QsAAAAAAgADABFRYBFP8A9KQT9LzyyAsCAgEgAxACAUgEBwLm0AHQ0wMhcbCSXwTgItdJwSCSXwTgAtMfIYIQcGx1Z70ighBkc3RyvbCSXwXgA/pAMCD6RAHIygfL/8nQ7UTQgQFA1yH0BDBcgQEI9ApvoTGzkl8H4AXTP8glghBwbHVnupI4MOMNA4IQZHN0crqSXwbjDQUGAHgB+gD0BDD4J28iMFAKoSG+8uBQghBwbHVngx6xcIAYUATLBSbPFlj6Ahn0AMtpF8sfUmDLPyDJgED7AAYAilAEgQEI9Fkw7UTQgQFA1yDIAc8W9ADJ7VQBcrCOI4IQZHN0coMesXCAGFAFywVQA88WI/oCE8tqyx/LP8mAQPsAkl8D4gIBIAgPAgEgCQ4CAVgKCwA9sp37UTQgQFA1yH0BDACyMoHy//J0AGBAQj0Cm+hMYAIBIAwNABmtznaiaEAga5Drhf/AABmvHfaiaEAQa5DrhY/AABG4yX7UTQ1wsfgAWb0kK29qJoQICga5D6AhhHDUCAhHpJN9KZEM5pA+n/mDeBKAG3gQFImHFZ8xhAT48oMI1xgg0x/TH9MfAvgju/Jk7UTQ0x/TH9P/9ATRUUO68qFRUbryogX5AVQQZPkQ8qP4ACSkyMsfUkDLH1Iwy/9SEPQAye1U+A8B0wchwACfbFGTINdKltMH1AL7AOgw4CHAAeMAIcAC4wABwAORMOMNA6TIyx8Syx/L/xESExQAbtIH+gDU1CL5AAXIygcVy//J0Hd0gBjIywXLAiLPFlAF+gIUy2sSzMzJc/sAyEAUgQEI9FHypwIAcIEBCNcY+gDTP8hUIEeBAQj0UfKnghBub3RlcHSAGMjLBcsCUAbPFlAE+gIUy2oSyx/LP8lz+wACAGyBAQjXGPoA0z8wUiSBAQj0WfKnghBkc3RycHSAGMjLBcsCUAXPFlAD+gITy2rLHxLLP8lz+wAACvQAye1UAFEAAAAAKamjFzHevlXTfHInaLE3ExyqYIcICy4LYLlL14XRRXXPpJi8QABoYgBB7+qpcxuU2jl+XmRiL15jNIuBKsW0djqT8N0gHQeY1CAX14QAAAAAAAAAAAAAAAAAADdEcGc="
    );

    let parsed = Message::from_boc(&boc).unwrap();
    let init = parsed.init.expect("deploy message must carry a StateInit");
    let code = init.value.code.expect("StateInit must carry code").0;
    let data = init.value.data.expect("StateInit must carry data").0;
    assert_eq!(tl_hash(&code), WalletVersion::V4R2.code_hash());
    let derived = TlAddress::derive(0, code, data).unwrap();
    assert_eq!(tx.wallet_address().to_string(), derived.to_base64_url());
    let CommonMsgInfo::ExtIn(info) = &parsed.info else {
        panic!("expected an external-in message");
    };
    let dest = TlAddress::from_msg_address(info.dest.clone()).unwrap();
    assert_eq!(derived, dest);
}

/// A v5r1 transfer with a text comment survives the round trip: tonlib-core
/// finds the action, the mode carries the forced IGNORE_ERRORS flag, the
/// inner MessageRelaxed matches our encoder, and the comment text is intact.
#[test]
fn test_v5r1_comment_transfer_round_trip() {
    let key = signing_key();
    let comment = "omni-transaction differential test";
    let message = transfer().with_comment(comment).unwrap();
    let tx = TransactionBuilder::new::<TON>()
        .public_key(public_key())
        .seqno(7)
        .valid_until(VALID_UNTIL)
        .add_message(message.clone())
        .build();

    let signature = sign(&tx, &key);
    let boc = tx.build_with_signature(signature);
    let parsed = Message::from_boc(&boc).unwrap();

    // Walk body -> OutList node -> [prev, message].
    let body = parsed.body.value;
    let out_list = body.references()[0].clone();
    let mut parser = out_list.parser();
    assert_eq!(parser.load_u32(32).unwrap(), 0x0EC3_C86D);
    // DEFAULT_SEND_MODE already includes IGNORE_ERRORS; the flag must stay.
    let mode = parser.load_u8(8).unwrap();
    assert_eq!(mode & 2, 2);
    assert_eq!(mode, message.mode | 2);
    let prev = out_list.references()[0].clone();
    assert_eq!(prev.bit_len(), 0);
    let inner = out_list.references()[1].clone();
    assert_eq!(tl_hash(&inner), message.to_cell().unwrap().repr_hash());

    // The comment (opcode 0 + UTF-8) sits at the end of the inlined body.
    let comment_bits = 32 + comment.len() * 8;
    let mut parser = inner.parser();
    let _ = parser.load_bits(inner.bit_len() - comment_bits).unwrap();
    assert_eq!(parser.load_u32(32).unwrap(), 0);
    assert_eq!(parser.load_utf8(comment.len()).unwrap(), comment);

    // Signature still verifies over the recomputed unsigned-body hash.
    let recomputed = tl_rebuild_hash(&body, 0, 512);
    assert_eq!(recomputed.to_vec(), tx.build_for_signing());
    let extracted = tl_extract_bits(&body, body.bit_len() - 512, 512);
    key.verifying_key()
        .verify(&recomputed, &Signature::from_slice(&extracted).unwrap())
        .unwrap();
}

/// Multi-message v5r1 transactions produce one OutList node per transfer,
/// innermost-first, all parseable by tonlib-core.
#[test]
fn test_v5r1_multi_message_out_list() {
    let key = signing_key();
    let first = transfer();
    let second = transfer().with_comment("second").unwrap();
    let tx = TransactionBuilder::new::<TON>()
        .public_key(public_key())
        .seqno(2)
        .valid_until(VALID_UNTIL)
        .messages(vec![first.clone(), second.clone()])
        .build();

    let signature = sign(&tx, &key);
    let boc = tx.build_with_signature(signature);
    let parsed = Message::from_boc(&boc).unwrap();
    let body = parsed.body.value;

    // Outermost node is the LAST message; its prev holds the first.
    let outer = body.references()[0].clone();
    assert_eq!(
        tl_hash(&outer.references()[1]),
        second.to_cell().unwrap().repr_hash()
    );
    let node = outer.references()[0].clone();
    assert_eq!(
        tl_hash(&node.references()[1]),
        first.to_cell().unwrap().repr_hash()
    );
    assert_eq!(node.references()[0].bit_len(), 0); // out_list_empty
}
