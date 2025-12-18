// Rust Bitcoin
use bitcoin::consensus::Encodable;
use bitcoin::hashes::{sha256d, Hash};
use bitcoin::script::Builder;
use bitcoin::secp256k1::{Message, Secp256k1};
use bitcoin::sighash::SighashCache;
use bitcoin::{absolute, transaction, Amount, Transaction, TxIn};
use bitcoin::{EcdsaSighashType, OutPoint};
use bitcoin::{ScriptBuf, Sequence};
use bitcoin::{TxOut, Witness};
// Omni testing utilities
use omni_testing_utilities::bitcoin::setup_bitcoin_testnet;
use omni_testing_utilities::bitcoin::BTCTestContext;
use omni_testing_utilities::bitcoind::AddressType;
// Omni library
use omni_transaction::bitcoin::types::{
    Amount as OmniAmount, EcdsaSighashType as OmniSighashType, Hash as OmniHash,
    LockTime as OmniLockTime, OutPoint as OmniOutPoint, ScriptBuf as OmniScriptBuf,
    Sequence as OmniSequence, TransactionType, TxIn as OmniTxIn, TxOut as OmniTxOut,
    Txid as OmniTxid, Version as OmniVersion, Witness as OmniWitness,
};
use omni_transaction::bitcoin::BitcoinTransaction;
use omni_transaction::TransactionBuilder;
use omni_transaction::TxBuilder;
use omni_transaction::BITCOIN;
// Testing
use eyre::Result;
use serde_json::json;
use std::result::Result::Ok;

const OMNI_SPEND_AMOUNT: OmniAmount = OmniAmount::from_sat(500_000_000);
const BITCOIN_SPEND_AMOUNT: Amount = Amount::from_sat(500_000_000);

#[tokio::test]
async fn test_sighash_p2wpkh_using_rust_bitcoin_and_omni_library() -> Result<()> {
    let bitcoind = setup_bitcoin_testnet().unwrap();
    let client = &bitcoind.client;

    // Setup testing environment
    let btc_test_context = BTCTestContext::new(client).unwrap();

    // Setup Bob and Alice addresses
    let bob = btc_test_context.setup_account(AddressType::Bech32).unwrap();

    let alice = btc_test_context.setup_account(AddressType::Bech32).unwrap();

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
    let bitcoin_txid: bitcoin::Txid = txid_str.parse()?;
    let omni_hash = OmniHash::from_hex(txid_str)?;
    let omni_txid = OmniTxid(omni_hash);

    assert_eq!(bitcoin_txid.to_string(), omni_txid.to_string());

    let vout = first_unspent["vout"].as_u64().unwrap();

    // Create inputs using Omni library
    let txin: OmniTxIn = OmniTxIn {
        previous_output: OmniOutPoint::new(omni_txid, vout as u32),
        script_sig: OmniScriptBuf::default(), // For a p2wpkh script_sig is empty.
        sequence: OmniSequence::ENABLE_RBF_NO_LOCKTIME,
        witness: OmniWitness::default(), // Filled in after signing.
    };

    let utxo_amount =
        OmniAmount::from_sat((first_unspent["amount"].as_f64().unwrap() * 100_000_000.0) as u64);

    let change_amount: OmniAmount = utxo_amount - OMNI_SPEND_AMOUNT - OmniAmount::from_sat(1000); // 1000 satoshis for fee

    // The change output is locked to a key controlled by us.
    let change_txout = OmniTxOut {
        value: change_amount,
        script_pubkey: OmniScriptBuf(ScriptBuf::new_p2wpkh(&bob.wpkh).into_bytes()),
    };

    // The spend output is locked to a key controlled by the receiver. In this case to Alice.
    let spend_txout = OmniTxOut {
        value: OMNI_SPEND_AMOUNT,
        script_pubkey: OmniScriptBuf(alice.address.script_pubkey().into_bytes()),
    };

    let omni_tx: BitcoinTransaction = TransactionBuilder::new::<BITCOIN>()
        .version(OmniVersion::Two)
        .lock_time(OmniLockTime::from_height(0).unwrap())
        .inputs(vec![txin])
        .outputs(vec![spend_txout, change_txout])
        .build();

    let script_pubkey_bob = ScriptBuf::new_p2wpkh(&bob.wpkh)
        .p2wpkh_script_code()
        .unwrap();

    // Prepare the transaction for signing
    let sighash_type = OmniSighashType::All;
    let input_index = 0;
    let encoded_data = omni_tx.build_for_signing_segwit(
        sighash_type,
        input_index,
        &OmniScriptBuf(script_pubkey_bob.into_bytes()),
        utxo_amount.to_sat(),
    );

    // Calculate the sighash
    let sighash_omni = sha256d::Hash::hash(&encoded_data);

    // Calculate the sighash with Rust Bitcoin
    let input = TxIn {
        previous_output: OutPoint::new(bitcoin_txid, vout as u32),
        script_sig: ScriptBuf::default(), // For a p2wpkh script_sig is empty.
        sequence: Sequence::ENABLE_RBF_NO_LOCKTIME,
        witness: Witness::default(), // Filled in after signing.
    };

    // The spend output is locked to a key controlled by the receiver.
    let spend = TxOut {
        value: BITCOIN_SPEND_AMOUNT,
        script_pubkey: alice.address.script_pubkey(),
    };

    let btc_utxo_amount =
        Amount::from_sat((first_unspent["amount"].as_f64().unwrap() * 100_000_000.0) as u64);

    let btc_change_amount: Amount = btc_utxo_amount - BITCOIN_SPEND_AMOUNT - Amount::from_sat(1000); // 1000 satoshis for fee

    // The change output is locked to a key controlled by us.
    let change = TxOut {
        value: btc_change_amount,
        script_pubkey: ScriptBuf::new_p2wpkh(&bob.wpkh), // Change comes back to us.
    };

    // The transaction we want to sign and broadcast.
    let mut unsigned_tx = Transaction {
        version: transaction::Version::TWO,  // Post BIP-68.
        lock_time: absolute::LockTime::ZERO, // Ignore the locktime.
        input: vec![input],                  // Input goes into index 0.
        output: vec![spend, change],         // Outputs, order does not matter.
    };
    let input_index = 0;

    let mut buffer = Vec::new();
    unsigned_tx.consensus_encode(&mut buffer).unwrap();

    // Get the sighash to sign.
    let script_pubkey = ScriptBuf::new_p2wpkh(&bob.wpkh);

    let sighash_type = EcdsaSighashType::All;
    let mut sighasher = SighashCache::new(&mut unsigned_tx);

    let mut writer = Vec::new();
    let script_code = script_pubkey.p2wpkh_script_code().unwrap();

    sighasher
        .segwit_v0_encode_signing_data_to(
            &mut writer,
            input_index,
            &script_code,
            btc_utxo_amount,
            sighash_type,
        )
        .expect("failed to create sighash");

    let sighash_bitcoin = sighasher
        .p2wpkh_signature_hash(input_index, &script_pubkey, btc_utxo_amount, sighash_type)
        .expect("failed to create sighash");

    // Assert that the sighash is the same
    assert_eq!(
        sighash_omni.to_byte_array(),
        sighash_bitcoin.to_byte_array()
    );

    Ok(())
}

#[tokio::test]
async fn test_sighash_for_multiple_p2wpkh_utxos() -> Result<()> {
    let bitcoind = setup_bitcoin_testnet().unwrap();
    let client = &bitcoind.client;

    // Setup testing environment
    let btc_test_context = BTCTestContext::new(client).unwrap();

    // Setup Bob and Alice addresses
    let bob = btc_test_context.setup_account(AddressType::Bech32).unwrap();

    let alice = btc_test_context.setup_account(AddressType::Bech32).unwrap();

    // Generate 101 blocks to the address
    client.generate_to_address(101, &bob.address)?;
    client.generate_to_address(101, &bob.address)?;

    // List UTXOs for Bob
    let unspent_utxos_bob = btc_test_context.get_utxo_for_address(&bob.address).unwrap();

    // Get the first two UTXOs
    let first_two_unspent: Vec<_> = unspent_utxos_bob.into_iter().take(2).collect();
    assert!(
        first_two_unspent.len() == 2,
        "There should be at least two unspent outputs"
    );

    let mut inputs = Vec::new();
    let mut total_utxo_amount = 0;
    let mut bitcoin_txids = Vec::new();

    for unspent in &first_two_unspent {
        let txid_str = unspent["txid"].as_str().unwrap();
        let bitcoin_txid: bitcoin::Txid = txid_str.parse()?;
        let omni_hash = OmniHash::from_hex(txid_str)?;
        let omni_txid = OmniTxid(omni_hash);

        assert_eq!(bitcoin_txid.to_string(), omni_txid.to_string());

        let vout = unspent["vout"].as_u64().unwrap();

        // Create inputs using Omni library
        let txin: OmniTxIn = OmniTxIn {
            previous_output: OmniOutPoint::new(omni_txid, vout as u32),
            script_sig: OmniScriptBuf::default(), // For a p2wpkh script_sig is empty.
            sequence: OmniSequence::ENABLE_RBF_NO_LOCKTIME,
            witness: OmniWitness::default(), // Filled in after signing.
        };

        let btc_txid = TxIn {
            previous_output: OutPoint::new(bitcoin_txid, vout as u32),
            script_sig: ScriptBuf::default(), // For a p2wpkh script_sig is empty.
            sequence: Sequence::ENABLE_RBF_NO_LOCKTIME,
            witness: Witness::default(), // Filled in after signing.
        };

        inputs.push(txin);
        bitcoin_txids.push(btc_txid);

        let utxo_amount =
            OmniAmount::from_sat((unspent["amount"].as_f64().unwrap() * 100_000_000.0) as u64);

        total_utxo_amount += utxo_amount.to_sat();
    }

    let change_amount: OmniAmount =
        OmniAmount::from_sat(total_utxo_amount) - OMNI_SPEND_AMOUNT - OmniAmount::from_sat(1000); // 1000 satoshis for fee

    // The change output is locked to a key controlled by us.
    let change_txout = OmniTxOut {
        value: change_amount,
        script_pubkey: OmniScriptBuf(ScriptBuf::new_p2wpkh(&bob.wpkh).into_bytes()),
    };

    let btc_change_amount = Amount::from_sat(change_amount.to_sat());

    // BTC Change Output
    let btc_change_txout = TxOut {
        value: btc_change_amount,
        script_pubkey: ScriptBuf::new_p2wpkh(&bob.wpkh), // Change comes back to us.
    };

    // BTC Spend Output
    let btc_spend_txout = TxOut {
        value: BITCOIN_SPEND_AMOUNT,
        script_pubkey: alice.address.script_pubkey(),
    };

    // The spend output is locked to a key controlled by the receiver. In this case to Alice.
    let spend_txout = OmniTxOut {
        value: OMNI_SPEND_AMOUNT,
        script_pubkey: OmniScriptBuf(alice.address.script_pubkey().into_bytes()),
    };

    let omni_tx: BitcoinTransaction = TransactionBuilder::new::<BITCOIN>()
        .version(OmniVersion::Two)
        .lock_time(OmniLockTime::from_height(0).unwrap())
        .inputs(inputs)
        .outputs(vec![spend_txout, change_txout])
        .build();

    // BTC Transaction
    let mut unsigned_tx = Transaction {
        version: transaction::Version::TWO,              // Post BIP-68.
        lock_time: absolute::LockTime::ZERO,             // Ignore the locktime.
        input: bitcoin_txids,                            // Input goes into index 0.
        output: vec![btc_spend_txout, btc_change_txout], // Outputs, order does not matter.
    };

    let mut buffer = Vec::new();
    unsigned_tx.consensus_encode(&mut buffer).unwrap();

    assert_eq!(buffer, omni_tx.serialize());

    for (i, unspent) in first_two_unspent.iter().enumerate() {
        let utxo_amount =
            OmniAmount::from_sat((unspent["amount"].as_f64().unwrap() * 100_000_000.0) as u64);

        let script_pub_key = ScriptBuf::new_p2wpkh(&bob.wpkh);
        let script_pubkey_bob = script_pub_key.p2wpkh_script_code().unwrap();

        // Prepare the transaction for signing
        let sighash_type = OmniSighashType::All;
        let input_index = i;
        let encoded_data = omni_tx.build_for_signing_segwit(
            sighash_type,
            input_index,
            &OmniScriptBuf(script_pubkey_bob.into_bytes()),
            utxo_amount.to_sat(),
        );

        // Calculate the sighash
        let sighash_omni = sha256d::Hash::hash(&encoded_data);

        let btc_sighash_type = EcdsaSighashType::All;
        let mut sighasher = SighashCache::new(&mut unsigned_tx);

        let btc_utxo_amount = Amount::from_sat(utxo_amount.to_sat());

        let mut encoded_btc_sighash = Vec::new();

        sighasher
            .segwit_v0_encode_signing_data_to(
                &mut encoded_btc_sighash,
                input_index,
                &script_pub_key.p2wpkh_script_code().unwrap(),
                btc_utxo_amount,
                btc_sighash_type,
            )
            .expect("failed to create sighash");

        assert_eq!(encoded_btc_sighash, encoded_data);

        let sighash_bitcoin = sighasher
            .p2wpkh_signature_hash(
                input_index,
                &script_pub_key,
                btc_utxo_amount,
                btc_sighash_type,
            )
            .expect("failed to create sighash");

        assert_eq!(
            sighash_omni.to_byte_array(),
            sighash_bitcoin.to_byte_array(),
            "SIGHASHES ARE NOT THE SAME"
        );
    }

    Ok(())
}

#[tokio::test]
async fn test_send_p2pkh_using_rust_bitcoin_and_omni_library() -> Result<()> {
    let bitcoind = setup_bitcoin_testnet().unwrap();
    let client = &bitcoind.client;

    // Setup testing environment
    let btc_test_context = BTCTestContext::new(client).unwrap();

    // Setup Bob and Alice addresses
    let bob = btc_test_context.setup_account(AddressType::Legacy).unwrap();

    let alice = btc_test_context.setup_account(AddressType::Legacy).unwrap();

    // Generate 101 blocks to Bob's address
    client.generate_to_address(101, &bob.address)?;

    // List UTXOs for Bob
    let unspent_utxos_bob = btc_test_context.get_utxo_for_address(&bob.address).unwrap();

    // Get the first UTXO
    let first_unspent = unspent_utxos_bob
        .into_iter()
        .next()
        .expect("There should be at least one unspent output");

    let txid_str = first_unspent["txid"].as_str().unwrap();
    let bitcoin_txid: bitcoin::Txid = txid_str.parse()?;
    let omni_hash = OmniHash::from_hex(txid_str)?;
    let omni_txid = OmniTxid(omni_hash);

    assert_eq!(bitcoin_txid.to_string(), omni_txid.to_string());

    let vout = first_unspent["vout"].as_u64().unwrap();

    // Create inputs using Omni library
    let txin: OmniTxIn = OmniTxIn {
        previous_output: OmniOutPoint::new(omni_txid, vout as u32),
        script_sig: OmniScriptBuf::default(), // Initially empty, will be filled later with the signature
        sequence: OmniSequence::MAX,
        witness: OmniWitness::default(),
    };

    // Create the spend output
    let txout = OmniTxOut {
        value: OMNI_SPEND_AMOUNT,
        script_pubkey: OmniScriptBuf(alice.script_pubkey.as_bytes().to_vec()),
    };

    let utxo_amount =
        OmniAmount::from_sat((first_unspent["amount"].as_f64().unwrap() * 100_000_000.0) as u64);

    let change_amount: OmniAmount = utxo_amount - OMNI_SPEND_AMOUNT - OmniAmount::from_sat(1000); // 1000 satoshis for fee

    // Create the change output
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

    // Encode the transaction for signing
    let sighash_type = OmniSighashType::All;
    let encoded_data = omni_tx.build_for_signing_legacy(sighash_type);

    // Calculate the sighash
    let sighash_omni = sha256d::Hash::hash(&encoded_data);
    let msg_omni = Message::from_digest_slice(sighash_omni.as_byte_array()).unwrap();

    // Sign the sighash and broadcast the transaction using the Omni library
    let secp = Secp256k1::new();
    let signature_omni = secp.sign_ecdsa(&msg_omni, &bob.private_key);

    // Verify signature
    let is_valid = secp
        .verify_ecdsa(&msg_omni, &signature_omni, &bob.public_key)
        .is_ok();

    assert!(is_valid, "The signature should be valid");

    // Encode the signature
    let signature = bitcoin::ecdsa::Signature {
        signature: signature_omni,
        sighash_type: EcdsaSighashType::All,
    };

    // Create the script_sig
    let script_sig_new = Builder::new()
        .push_slice(signature.serialize())
        .push_key(&bob.bitcoin_public_key)
        .into_script();

    // Assign script_sig to txin
    let omni_script_sig = OmniScriptBuf(script_sig_new.as_bytes().to_vec());
    let encoded_omni_tx = omni_tx.build_with_script_sig(0, omni_script_sig, TransactionType::P2PKH);

    // Convert the transaction to a hexadecimal string
    let hex_omni_tx = hex::encode(encoded_omni_tx);

    let raw_tx_result: serde_json::Value = client
        .call("sendrawtransaction", &[json!(hex_omni_tx)])
        .unwrap();

    println!("raw_tx_result: {raw_tx_result:?}");

    client.generate_to_address(101, &bob.address)?;

    btc_test_context.assert_utxos_for_address(alice.address, 1);

    Ok(())
}

#[tokio::test]
async fn test_p2wpkh_single_utxo() -> Result<()> {
    let bitcoind = setup_bitcoin_testnet().unwrap();
    let client = &bitcoind.client;

    // Setup testing environment
    let btc_test_context = BTCTestContext::new(client).unwrap();

    // Setup Bob and Alice addresses
    let bob = btc_test_context.setup_account(AddressType::Bech32).unwrap();

    let alice = btc_test_context.setup_account(AddressType::Bech32).unwrap();

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
    let bitcoin_txid: bitcoin::Txid = txid_str.parse()?;
    let omni_hash = OmniHash::from_hex(txid_str)?;
    let omni_txid = OmniTxid(omni_hash);

    assert_eq!(bitcoin_txid.to_string(), omni_txid.to_string());

    let vout = first_unspent["vout"].as_u64().unwrap();

    // Create inputs using Omni library
    let txin: OmniTxIn = OmniTxIn {
        previous_output: OmniOutPoint::new(omni_txid, vout as u32),
        script_sig: OmniScriptBuf::default(), // For a p2wpkh script_sig is empty.
        sequence: OmniSequence::ENABLE_RBF_NO_LOCKTIME,
        witness: OmniWitness::default(), // Filled in after signing.
    };

    let utxo_amount =
        OmniAmount::from_sat((first_unspent["amount"].as_f64().unwrap() * 100_000_000.0) as u64);

    let change_amount: OmniAmount = utxo_amount - OMNI_SPEND_AMOUNT - OmniAmount::from_sat(1000); // 1000 satoshis for fee

    // The change output is locked to a key controlled by us.
    let change_txout = OmniTxOut {
        value: change_amount,
        script_pubkey: OmniScriptBuf(ScriptBuf::new_p2wpkh(&bob.wpkh).into_bytes()),
    };

    // The spend output is locked to a key controlled by the receiver. In this case to Alice.
    let spend_txout = OmniTxOut {
        value: OMNI_SPEND_AMOUNT,
        script_pubkey: OmniScriptBuf(alice.address.script_pubkey().into_bytes()),
    };

    let mut omni_tx: BitcoinTransaction = TransactionBuilder::new::<BITCOIN>()
        .version(OmniVersion::Two)
        .lock_time(OmniLockTime::from_height(0).unwrap())
        .inputs(vec![txin])
        .outputs(vec![spend_txout, change_txout])
        .build();

    let script_pubkey_bob = ScriptBuf::new_p2wpkh(&bob.wpkh)
        .p2wpkh_script_code()
        .unwrap();

    // Prepare the transaction for signing
    let sighash_type = OmniSighashType::All;
    let input_index = 0;
    let encoded_data = omni_tx.build_for_signing_segwit(
        sighash_type,
        input_index,
        &OmniScriptBuf(script_pubkey_bob.into_bytes()),
        utxo_amount.to_sat(),
    );

    // Calculate the sighash
    let sighash_omni = sha256d::Hash::hash(&encoded_data);

    // Calculate the sighash with Rust Bitcoin
    let input = TxIn {
        previous_output: OutPoint::new(bitcoin_txid, vout as u32),
        script_sig: ScriptBuf::default(), // For a p2wpkh script_sig is empty.
        sequence: Sequence::ENABLE_RBF_NO_LOCKTIME,
        witness: Witness::default(), // Filled in after signing.
    };

    // The spend output is locked to a key controlled by the receiver.
    let spend = TxOut {
        value: BITCOIN_SPEND_AMOUNT,
        script_pubkey: alice.address.script_pubkey(),
    };

    let btc_utxo_amount =
        Amount::from_sat((first_unspent["amount"].as_f64().unwrap() * 100_000_000.0) as u64);

    let btc_change_amount: Amount = btc_utxo_amount - BITCOIN_SPEND_AMOUNT - Amount::from_sat(1000); // 1000 satoshis for fee

    // The change output is locked to a key controlled by us.
    let change = TxOut {
        value: btc_change_amount,
        script_pubkey: ScriptBuf::new_p2wpkh(&bob.wpkh), // Change comes back to us.
    };

    // The transaction we want to sign and broadcast.
    let mut unsigned_tx = Transaction {
        version: transaction::Version::TWO,  // Post BIP-68.
        lock_time: absolute::LockTime::ZERO, // Ignore the locktime.
        input: vec![input],                  // Input goes into index 0.
        output: vec![spend, change],         // Outputs, order does not matter.
    };
    let input_index = 0;

    let mut buffer = Vec::new();
    unsigned_tx.consensus_encode(&mut buffer).unwrap();

    // Get the sighash to sign.
    let sighash_type = EcdsaSighashType::All;
    let mut sighasher = SighashCache::new(&mut unsigned_tx);
    let script_pubkey = ScriptBuf::new_p2wpkh(&bob.wpkh);
    let script_code = script_pubkey.p2wpkh_script_code().unwrap();

    let mut encoded_btc_sighash = Vec::new();

    sighasher
        .segwit_v0_encode_signing_data_to(
            &mut encoded_btc_sighash,
            input_index,
            &script_code,
            btc_utxo_amount,
            sighash_type,
        )
        .expect("failed to create sighash");

    assert_eq!(encoded_btc_sighash, encoded_data);

    let sighash_bitcoin = sighasher
        .p2wpkh_signature_hash(input_index, &script_pubkey, btc_utxo_amount, sighash_type)
        .expect("failed to create sighash");

    // Assert that the sighash is the same
    assert_eq!(
        sighash_omni.to_byte_array(),
        sighash_bitcoin.to_byte_array()
    );

    let msg_omni = Message::from_digest_slice(sighash_omni.as_byte_array()).unwrap();

    // Sign the sighash and broadcast the transaction using the Omni library
    let secp = Secp256k1::new();
    let signature_omni = secp.sign_ecdsa(&msg_omni, &bob.private_key);

    // Verify signature
    let is_valid = secp
        .verify_ecdsa(&msg_omni, &signature_omni, &bob.public_key)
        .is_ok();

    assert!(is_valid, "The signature should be valid");

    // Encode the signature
    let signature = bitcoin::ecdsa::Signature {
        signature: signature_omni,
        sighash_type: EcdsaSighashType::All,
    };

    // Create the witness
    let witness = Witness::p2wpkh(&signature, &bob.public_key);

    let encoded_omni_tx = omni_tx.build_with_witness(0, witness.to_vec(), TransactionType::P2WPKH);

    // Convert the transaction to a hexadecimal string
    let hex_omni_tx = hex::encode(encoded_omni_tx);

    let maxfeerate = 0.10;
    let maxburnamount = 100.00;

    // We now deploy to the bitcoin network (regtest mode)
    let raw_tx_result: serde_json::Value = client
        .call(
            "sendrawtransaction",
            &[json!(hex_omni_tx), json!(maxfeerate), json!(maxburnamount)],
        )
        .unwrap();

    println!("raw_tx_result: {raw_tx_result:?}");

    client.generate_to_address(101, &bob.address)?;

    Ok(())
}

#[tokio::test]
async fn test_p2wpkh_multiple_utxos() -> Result<()> {
    let bitcoind = setup_bitcoin_testnet().unwrap();
    let client = &bitcoind.client;

    // Setup testing environment
    let btc_test_context = BTCTestContext::new(client).unwrap();

    // Setup Bob and Alice addresses
    let bob = btc_test_context.setup_account(AddressType::Bech32).unwrap();

    let alice = btc_test_context.setup_account(AddressType::Bech32).unwrap();

    // Generate 101 blocks to the address
    client.generate_to_address(101, &bob.address)?;
    client.generate_to_address(101, &bob.address)?;

    // List UTXOs for Bob
    let unspent_utxos_bob = btc_test_context.get_utxo_for_address(&bob.address).unwrap();

    // Get the first two UTXOs
    let first_two_unspent: Vec<_> = unspent_utxos_bob.into_iter().take(2).collect();
    assert!(
        first_two_unspent.len() == 2,
        "There should be at least two unspent outputs"
    );

    let mut inputs = Vec::new();
    let mut total_utxo_amount = 0;
    let mut bitcoin_txids = Vec::new();

    for unspent in &first_two_unspent {
        let txid_str = unspent["txid"].as_str().unwrap();
        let bitcoin_txid: bitcoin::Txid = txid_str.parse()?;
        let omni_hash = OmniHash::from_hex(txid_str)?;
        let omni_txid = OmniTxid(omni_hash);

        assert_eq!(bitcoin_txid.to_string(), omni_txid.to_string());

        let vout = unspent["vout"].as_u64().unwrap();

        // Create inputs using Omni library
        let txin: OmniTxIn = OmniTxIn {
            previous_output: OmniOutPoint::new(omni_txid, vout as u32),
            script_sig: OmniScriptBuf::default(), // For a p2wpkh script_sig is empty.
            sequence: OmniSequence::ENABLE_RBF_NO_LOCKTIME,
            witness: OmniWitness::default(), // Filled in after signing.
        };

        let btc_txid = TxIn {
            previous_output: OutPoint::new(bitcoin_txid, vout as u32),
            script_sig: ScriptBuf::default(), // For a p2wpkh script_sig is empty.
            sequence: Sequence::ENABLE_RBF_NO_LOCKTIME,
            witness: Witness::default(), // Filled in after signing.
        };

        inputs.push(txin);
        bitcoin_txids.push(btc_txid);

        let utxo_amount =
            OmniAmount::from_sat((unspent["amount"].as_f64().unwrap() * 100_000_000.0) as u64);

        total_utxo_amount += utxo_amount.to_sat();
    }

    let change_amount: OmniAmount =
        OmniAmount::from_sat(total_utxo_amount) - OMNI_SPEND_AMOUNT - OmniAmount::from_sat(1000); // 1000 satoshis for fee

    // The change output is locked to a key controlled by us.
    let change_txout = OmniTxOut {
        value: change_amount,
        script_pubkey: OmniScriptBuf(ScriptBuf::new_p2wpkh(&bob.wpkh).into_bytes()),
    };

    let btc_change_amount = Amount::from_sat(change_amount.to_sat());

    // BTC Change Output
    let btc_change_txout = TxOut {
        value: btc_change_amount,
        script_pubkey: ScriptBuf::new_p2wpkh(&bob.wpkh), // Change comes back to us.
    };

    // BTC Spend Output
    let btc_spend_txout = TxOut {
        value: BITCOIN_SPEND_AMOUNT,
        script_pubkey: alice.address.script_pubkey(),
    };

    // The spend output is locked to a key controlled by the receiver. In this case to Alice.
    let spend_txout = OmniTxOut {
        value: OMNI_SPEND_AMOUNT,
        script_pubkey: OmniScriptBuf(alice.address.script_pubkey().into_bytes()),
    };

    let mut omni_tx: BitcoinTransaction = TransactionBuilder::new::<BITCOIN>()
        .version(OmniVersion::Two)
        .lock_time(OmniLockTime::from_height(0).unwrap())
        .inputs(inputs)
        .outputs(vec![spend_txout, change_txout])
        .build();

    // BTC Transaction
    let mut unsigned_tx = Transaction {
        version: transaction::Version::TWO,              // Post BIP-68.
        lock_time: absolute::LockTime::ZERO,             // Ignore the locktime.
        input: bitcoin_txids,                            // Input goes into index 0.
        output: vec![btc_spend_txout, btc_change_txout], // Outputs, order does not matter.
    };

    let mut buffer = Vec::new();
    unsigned_tx.consensus_encode(&mut buffer).unwrap();

    assert_eq!(buffer, omni_tx.serialize());

    let secp = Secp256k1::new();

    let mut witness_vec = Vec::new();
    for (i, unspent) in first_two_unspent.iter().enumerate() {
        let utxo_amount =
            OmniAmount::from_sat((unspent["amount"].as_f64().unwrap() * 100_000_000.0) as u64);

        let script_pub_key = ScriptBuf::new_p2wpkh(&bob.wpkh);
        let script_pubkey_bob = script_pub_key.p2wpkh_script_code().unwrap();

        // Prepare the transaction for signing
        let sighash_type = OmniSighashType::All;
        let input_index = i;
        let encoded_data = omni_tx.build_for_signing_segwit(
            sighash_type,
            input_index,
            &OmniScriptBuf(script_pubkey_bob.into_bytes()),
            utxo_amount.to_sat(),
        );

        // Calculate the sighash
        let sighash_omni = sha256d::Hash::hash(&encoded_data);

        let btc_sighash_type = EcdsaSighashType::All;
        let mut sighasher = SighashCache::new(&mut unsigned_tx);

        let btc_utxo_amount = Amount::from_sat(utxo_amount.to_sat());

        let mut encoded_btc_sighash = Vec::new();

        sighasher
            .segwit_v0_encode_signing_data_to(
                &mut encoded_btc_sighash,
                input_index,
                &script_pub_key.p2wpkh_script_code().unwrap(),
                btc_utxo_amount,
                btc_sighash_type,
            )
            .expect("failed to create sighash");

        assert_eq!(encoded_btc_sighash, encoded_data);

        let sighash_bitcoin = sighasher
            .p2wpkh_signature_hash(
                input_index,
                &script_pub_key,
                btc_utxo_amount,
                btc_sighash_type,
            )
            .expect("failed to create sighash");

        assert_eq!(
            sighash_omni.to_byte_array(),
            sighash_bitcoin.to_byte_array(),
            "SIGHASHES ARE NOT THE SAME"
        );

        // Sign the sighash
        let msg_omni = Message::from_digest_slice(sighash_omni.as_byte_array()).unwrap();
        let signature_omni = secp.sign_ecdsa(&msg_omni, &bob.private_key);

        // Verify signature
        let is_valid = secp
            .verify_ecdsa(&msg_omni, &signature_omni, &bob.public_key)
            .is_ok();

        assert!(is_valid, "The signature should be valid");

        // Encode the signature
        let signature = bitcoin::ecdsa::Signature {
            signature: signature_omni,
            sighash_type: EcdsaSighashType::All,
        };

        // Create the witness
        let witness = Witness::p2wpkh(&signature, &bob.public_key);

        witness_vec.push(omni_transaction::bitcoin::types::Witness::from_slice(
            &witness.to_vec(),
        ));
    }

    // Add the witness to the transaction
    for (i, witness) in witness_vec.iter().enumerate() {
        omni_tx.input[i].witness = witness.clone();
    }

    // Serialize the transaction
    let serialized_tx = omni_tx.serialize();

    // Convert the transaction to a hexadecimal string
    let hex_omni_tx = hex::encode(serialized_tx);

    let _decoded_tx: serde_json::Value =
        client.call("decoderawtransaction", &[json!(hex_omni_tx)])?;

    let maxfeerate = 0.10;
    let maxburnamount = 100.00;

    // We now deploy to the bitcoin network (regtest mode)
    let raw_tx_result: serde_json::Value = client
        .call(
            "sendrawtransaction",
            &[json!(hex_omni_tx), json!(maxfeerate), json!(maxburnamount)],
        )
        .unwrap();

    println!("raw_tx_result: {raw_tx_result:?}");

    client.generate_to_address(101, &bob.address)?;

    Ok(())
}
