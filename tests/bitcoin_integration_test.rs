// Rust Bitcoin
use bitcoin::hashes::{sha256d, Hash};
use bitcoin::script::{Builder, Instruction};
use bitcoin::secp256k1::{Message, Secp256k1};
use bitcoin::sighash::SighashCache;
use bitcoin::EcdsaSighashType;
use bitcoin::{
    absolute::Height, locktime::absolute::LockTime, transaction, Address, Amount, OutPoint,
    ScriptBuf, Sequence, Transaction, TxIn, TxOut, Witness,
};
// Omni library
use omni_transaction::bitcoin::bitcoin_transaction::BitcoinTransaction;
use omni_transaction::bitcoin::types::{
    Amount as OmniAmount, Hash as OmniHash, LockTime as OmniLockTime, OutPoint as OmniOutPoint,
    ScriptBuf as OmniScriptBuf, Sequence as OmniSequence, TxIn as OmniTxIn, TxOut as OmniTxOut,
    Txid as OmniTxid, Version as OmniVersion, Witness as OmniWitness,
};
use omni_transaction::transaction_builder::TransactionBuilder;
use omni_transaction::transaction_builder::TxBuilder;
use omni_transaction::types::BITCOIN;
// Testing
use eyre::Result;
use serde_json::json;
use std::result::Result::Ok;

mod utils;

pub use utils::bitcoin_utils::*;

const SPEND_AMOUNT: Amount = Amount::from_sat(500_000_000);
const OMNI_SPEND_AMOUNT: OmniAmount = OmniAmount::from_sat(500_000_000);

#[tokio::test]
async fn test_send_p2pkh_using_rust_bitcoin_and_omni_library() -> Result<()> {
    let bitcoind = bitcoind::BitcoinD::from_downloaded().unwrap();
    let client: &bitcoind::Client = &bitcoind.client;

    // Setup testing environment
    let mut btc_test_context = BTCTestContext::new(client).unwrap();

    // Setup Bob and Alice addresses
    let bob = btc_test_context.setup_account().unwrap();

    let alice = btc_test_context.setup_account().unwrap();

    // Generate 101 blocks to the address
    client.generate_to_address(101, &bob.address)?;

    // List UTXOs for Bob
    let unspent_utxos_bob = btc_test_context.get_utxo_for_address(&bob.address).unwrap();

    // Get the first UTXO
    let first_unspent = unspent_utxos_bob
        .into_iter()
        .next()
        .expect("There should be at least one unspent output");

    let txid_str = first_unspent["txid"].as_str().unwrap();
    println!("Original txid: {}", txid_str);

    let bitcoin_txid: bitcoin::Txid = txid_str.parse()?;
    println!("Bitcoin Txid: {:?}", bitcoin_txid);

    let omni_hash = OmniHash::from_hex(txid_str)?;
    let omni_txid = OmniTxid(omni_hash);
    println!("Omni Txid: {:?}", omni_txid);

    assert_eq!(bitcoin_txid.to_string(), omni_txid.to_string());

    let vout = first_unspent["vout"].as_u64().unwrap();

    // Create inputs using Omni library
    let txin: OmniTxIn = OmniTxIn {
        previous_output: OmniOutPoint::new(omni_txid, vout as u32),
        script_sig: OmniScriptBuf::default(), // Initially empty, will be filled later with the signature
        sequence: OmniSequence::MAX,
        witness: OmniWitness::default(),
    };

    println!("txin: {:?}", txin);

    let txout = OmniTxOut {
        value: OMNI_SPEND_AMOUNT,
        script_pubkey: OmniScriptBuf(alice.script_pubkey.as_bytes().to_vec()),
    };

    let utxo_amount =
        OmniAmount::from_sat((first_unspent["amount"].as_f64().unwrap() * 100_000_000.0) as u64);

    let change_amount: OmniAmount = utxo_amount - OMNI_SPEND_AMOUNT - OmniAmount::from_sat(1000); // 1000 satoshis for fee

    let change_txout = OmniTxOut {
        value: change_amount,
        script_pubkey: OmniScriptBuf(bob.script_pubkey.as_bytes().to_vec()),
    };

    let mut omni_tx: BitcoinTransaction = TransactionBuilder::new::<BITCOIN>()
        .version(OmniVersion::One)
        .lock_time(OmniLockTime::from_height(1).unwrap())
        .inputs(vec![txin])
        .outputs(vec![txout, change_txout])
        .build();

    // Add the script_sig to the transaction
    omni_tx.input[0].script_sig = OmniScriptBuf(bob.script_pubkey.as_bytes().to_vec());

    // Extend the transaction with the sighash type
    let sighash_type = EcdsaSighashType::All;
    let mut encoded_data = omni_tx.build_for_signing();
    let sighash_type_bytes = sighash_type.to_u32().to_le_bytes();
    encoded_data.extend_from_slice(&sighash_type_bytes);

    // Calculate the sighash
    let sighash_omni = sha256d::Hash::hash(&encoded_data);

    // -------------------------------------------------------------------------------------------------
    // Create an equivalent transaction using the Bitcoin library
    let txin_btc = TxIn {
        previous_output: OutPoint::new(
            first_unspent["txid"].as_str().unwrap().parse()?,
            first_unspent["vout"].as_u64().unwrap() as u32,
        ),
        script_sig: ScriptBuf::new(), // Initially empty, will be filled later with the signature
        sequence: Sequence::MAX,
        witness: Witness::default(),
    };

    let txout_btc = TxOut {
        value: SPEND_AMOUNT,
        script_pubkey: alice.script_pubkey.clone(),
    };

    let utxo_amount_btc = Amount::from_btc(first_unspent["amount"].as_f64().unwrap())?;
    let change_amount: Amount = utxo_amount_btc - SPEND_AMOUNT - Amount::from_sat(1000); // 1000 satoshis for fee

    let change_txout_btc = TxOut {
        value: change_amount,
        script_pubkey: bob.script_pubkey.clone(),
    };

    let mut tx_btc = Transaction {
        version: transaction::Version::ONE,
        lock_time: LockTime::Blocks(Height::from_consensus(1).unwrap()),
        input: vec![txin_btc],
        output: vec![txout_btc, change_txout_btc],
    };

    // Get the sighash to sign.
    let sighasher = SighashCache::new(&mut tx_btc);
    let sighash_btc = sighasher
        .legacy_signature_hash(0, &bob.script_pubkey, sighash_type.to_u32())
        .expect("failed to create sighash");

    println!("sighash btc: {:?}", sighash_btc);

    println!("sighash: {:?}", sighash_omni.to_byte_array());
    println!("sighash btc: {:?}", sighash_btc.to_byte_array());

    assert_eq!(
        sighash_omni.to_byte_array(),
        sighash_btc.to_byte_array(),
        "sighash btc is not equal to sighash"
    );

    // Sign the sighash and broadcast the transaction
    let secp = Secp256k1::new();
    let msg_btc = Message::from(sighash_btc);
    let signature_btc = secp.sign_ecdsa(&msg_btc, &bob.private_key);
    // println!("signature btc: {:?}", signature_btc);

    // Verify signature
    let is_valid = secp
        .verify_ecdsa(&msg_btc, &signature_btc, &bob.public_key)
        .is_ok();
    println!("Signature valid: {:?}", is_valid);

    assert!(is_valid, "The signature should be valid");

    let signature = bitcoin::ecdsa::Signature {
        signature: signature_btc,
        sighash_type,
    };

    // Create the script_sig
    let script_sig_btc = Builder::new()
        .push_slice(&signature.serialize())
        .push_key(&bob.bitcoin_public_key)
        .into_script();

    // Assign script_sig to txin
    tx_btc.input[0].script_sig = script_sig_btc;

    // Finalize the transaction
    let tx_signed_btc = tx_btc;
    let tx_signed_btc_hex = hex::encode(bitcoin::consensus::encode::serialize(&tx_signed_btc));

    // assert_eq!(new_tx_hex, tx_signed_btc_hex);

    // -------------------------------------------------------------------------------------------------

    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_send_p2pkh_using_rust_bitcoin() -> Result<()> {
    let bitcoind = bitcoind::BitcoinD::from_downloaded().unwrap();
    let client: &bitcoind::Client = &bitcoind.client;

    // Setup testing environment
    let mut btc_test_context = BTCTestContext::new(client).unwrap();

    // Setup Bob and Alice addresses
    let bob = btc_test_context.setup_account().unwrap();

    let alice = btc_test_context.setup_account().unwrap();

    // Generate 101 blocks to the address
    client.generate_to_address(101, &bob.address)?;

    // List UTXOs for Bob
    let unspent_utxos_bob = btc_test_context.get_utxo_for_address(&bob.address).unwrap();

    // Get the first UTXO
    let first_unspent = unspent_utxos_bob
        .into_iter()
        .next()
        .expect("There should be at least one unspent output");

    let utxo_amount = Amount::from_btc(first_unspent["amount"].as_f64().unwrap())?;
    println!("UTXO amount: {:?}", utxo_amount);

    let txin = TxIn {
        previous_output: OutPoint::new(
            first_unspent["txid"].as_str().unwrap().parse()?,
            first_unspent["vout"].as_u64().unwrap() as u32,
        ),
        script_sig: ScriptBuf::new(), // Initially empty, will be filled later with the signature
        sequence: Sequence::MAX,
        witness: Witness::default(),
    };

    let txout = TxOut {
        value: SPEND_AMOUNT,
        script_pubkey: alice.script_pubkey.clone(),
    };

    let change_amount: Amount = utxo_amount - SPEND_AMOUNT - Amount::from_sat(1000); // 1000 satoshis for fee

    let change_txout = TxOut {
        value: change_amount,
        script_pubkey: bob.script_pubkey.clone(),
    };

    let mut tx = Transaction {
        version: transaction::Version::ONE,
        lock_time: LockTime::Blocks(Height::from_consensus(1).unwrap()),
        input: vec![txin],
        output: vec![txout, change_txout],
    };

    // Get the sighash to sign.
    let sighash_type = EcdsaSighashType::All;
    let sighasher = SighashCache::new(&mut tx);
    let sighash = sighasher
        .legacy_signature_hash(0, &bob.script_pubkey, sighash_type.to_u32())
        .expect("failed to create sighash");

    let msg = Message::from(sighash);

    let secp = Secp256k1::new();

    let signature = secp.sign_ecdsa(&msg, &bob.private_key);

    // Verify signature
    let is_valid = secp.verify_ecdsa(&msg, &signature, &bob.public_key).is_ok();
    println!("Signature valid: {:?}", is_valid);

    assert!(is_valid, "The signature should be valid");

    let signature = bitcoin::ecdsa::Signature {
        signature,
        sighash_type,
    };

    // Create the script_sig
    let script_sig = Builder::new()
        .push_slice(&signature.serialize())
        .push_key(&bob.bitcoin_public_key)
        .into_script();

    // Verify the script_sig
    println!("script_sig: {:?}", script_sig);

    // Decode the script_sig to verify its contents
    let mut iter = script_sig.instructions().peekable();

    // Check the signature
    if let Some(Ok(Instruction::PushBytes(sig_bytes))) = iter.next() {
        println!("Signature in script_sig: {:?}", sig_bytes);

        assert_eq!(
            sig_bytes.as_bytes(),
            signature.serialize().to_vec().as_slice(),
            "Signature mismatch in script_sig"
        );
    } else {
        panic!("Expected signature push in script_sig");
    }

    // Check the public key
    if let Some(Ok(Instruction::PushBytes(pubkey_bytes))) = iter.next() {
        println!("Public key in script_sig: {:?}", pubkey_bytes);
        assert_eq!(
            pubkey_bytes.as_bytes(),
            bob.bitcoin_public_key.to_bytes(),
            "Public key mismatch in script_sig"
        );
    } else {
        panic!("Expected public key push in script_sig");
    }

    // Ensure there are no more instructions
    assert!(iter.next().is_none(), "Unexpected data in script_sig");

    println!("script_sig verification passed");

    // Assign script_sig to txin
    tx.input[0].script_sig = script_sig;

    // Finalize the transaction
    let tx_signed = tx;

    println!("tx_signed: {:?}", tx_signed);

    let raw_tx_result = client.send_raw_transaction(&tx_signed).unwrap();
    println!("raw_tx_result: {:?}", raw_tx_result);

    client.generate_to_address(101, &bob.address)?;

    assert_utxos_for_address(client, alice.address, 1);

    Ok(())
}

fn assert_utxos_for_address(client: &bitcoind::Client, address: Address, number_of_utxos: usize) {
    let min_conf = 1;
    let max_conf = 9999999;
    let include_unsafe = true;
    let query_options = json!({});

    let unspent_utxos: Vec<serde_json::Value> = client
        .call(
            "listunspent",
            &[
                json!(min_conf),
                json!(max_conf),
                json!(vec![address.to_string()]),
                json!(include_unsafe),
                query_options.clone(),
            ],
        )
        .unwrap();

    assert!(
        unspent_utxos.len() == number_of_utxos,
        "Expected {} UTXOs for address {}, but found {}",
        number_of_utxos,
        address.to_string(),
        unspent_utxos.len()
    );
}
