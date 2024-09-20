// Rust Bitcoin
use bitcoin::bip32::{ChildNumber, DerivationPath, Xpub};
use bitcoin::hashes::{sha256d, Hash};
use bitcoin::hex::FromHex;
use bitcoin::script::{Builder, Instruction};
use bitcoin::secp256k1::{Message, Secp256k1};
use bitcoin::sighash::SighashCache;
use bitcoin::EcdsaSighashType;
// use bitcoin::sighash::{EcdsaSighashType, SighashCache};
use bitcoin::{
    absolute::Height, locktime::absolute::LockTime, transaction, transaction::Version, Address,
    Amount, Network, OutPoint, ScriptBuf, Sequence, Transaction, TxIn, TxOut, Witness,
};
use bitcoind::VERSION;
// use bitcoind::Conf;
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

const SPEND_AMOUNT: Amount = Amount::from_sat(500_000_000);
const OMNI_SPEND_AMOUNT: OmniAmount = OmniAmount::from_sat(500_000_000);

mod utils;

pub use utils::bitcoin_utils::*;

#[tokio::test]
async fn test_send_p2pkh_using_rust_bitcoin_and_omni_library() -> Result<()> {
    let bitcoind = bitcoind::BitcoinD::from_downloaded().unwrap();
    let client: &bitcoind::Client = &bitcoind.client;

    // Setup Bob and Alice addresses
    let (bob_address, bob_script_pubkey) =
        get_address_info_for(client, "Bob").expect("Failed to get address info for Bob");

    let (alice_address, alice_script_pubkey) =
        get_address_info_for(client, "Alice").expect("Failed to get address info for Alice");

    // Get descriptors
    let master_key = get_master_key_of_regtest_node(client).expect("Failed to get master key");

    // Initialize secp256k1 context
    let secp = Secp256k1::new();

    // Derive child private key using path m/44h/1h/0h
    let path = "m/44h/1h/0h".parse::<DerivationPath>().unwrap();
    let child = master_key.derive_priv(&secp, &path).unwrap();
    // println!("Child at {}: {}", path, child);

    let xpub = Xpub::from_priv(&secp, &child);
    // println!("Public key at {}: {}", path, xpub);

    // Generate first P2PKH address at m/0/0
    let zero = ChildNumber::Normal { index: 0 };
    let public_key = xpub.derive_pub(&secp, &[zero, zero]).unwrap().public_key;
    let bitcoin_public_key = bitcoin::PublicKey::new(public_key);
    let derived_bob_address = Address::p2pkh(&bitcoin_public_key, Network::Regtest);

    assert_eq!(bob_address, derived_bob_address);

    // Get private key for first P2PKH address
    let first_priv_key = child
        .derive_priv(&secp, &DerivationPath::from(vec![zero, zero]))
        .unwrap()
        .private_key;

    println!(
        "Private key for first receiving address: {:?}",
        first_priv_key
    );

    // Generate 101 blocks to the address
    client.generate_to_address(101, &bob_address)?;

    // List UTXOs for Bob
    let min_conf = 1;
    let max_conf = 9999999;
    let include_unsafe = true;
    let query_options = json!({});
    let unspent_utxos_bob: Vec<serde_json::Value> = client.call(
        "listunspent",
        &[
            json!(min_conf),
            json!(max_conf),
            json!(vec![bob_address.to_string()]),
            json!(include_unsafe),
            query_options.clone(),
        ],
    )?;

    // println!("UTXOs for Bob: {:?}", unspent_utxos_bob);

    // Get the first UTXO
    let first_unspent = unspent_utxos_bob
        .into_iter()
        .next()
        .expect("There should be at least one unspent output");

    println!("First UTXO: {:?}", first_unspent);

    // Verify UTXO belongs to our address and has the correct amount
    assert_eq!(
        first_unspent["address"].as_str().unwrap(),
        bob_address.to_string(),
        "UTXO doesn't belong to Bob"
    );

    // Generate second (alice) P2PKH address at m/0/1
    let one = ChildNumber::Normal { index: 1 };
    let alice_public_key = xpub.derive_pub(&secp, &[zero, one]).unwrap().public_key;
    let alice_bitcoin_public_key = bitcoin::PublicKey::new(alice_public_key);
    let derived_alice_address = Address::p2pkh(&alice_bitcoin_public_key, Network::Regtest);
    println!("Alice derived address: {}", derived_alice_address);

    assert_eq!(alice_address, derived_alice_address);

    let txid = first_unspent["txid"].as_str().unwrap();
    println!("txid: {:?}", txid);
    let vout = first_unspent["vout"].as_u64().unwrap();
    println!("vout: {:?}", vout);
    let omni_hash = OmniHash::from_hex(txid)?;
    println!("omni_hash: {:?}", omni_hash);
    let omni_txid = OmniTxid(omni_hash);
    println!("omni_txid: {:?}", omni_txid);

    // Create inputs using Omni library
    let txin: OmniTxIn = OmniTxIn {
        previous_output: OmniOutPoint::new(omni_txid, vout as u32),
        script_sig: OmniScriptBuf::default(), // Initially empty, will be filled later with the signature
        sequence: OmniSequence::MAX,
        witness: OmniWitness::default(),
    };

    let txout = OmniTxOut {
        value: OMNI_SPEND_AMOUNT,
        script_pubkey: OmniScriptBuf(alice_script_pubkey.as_bytes().to_vec()),
    };

    println!("txout: {:?}", txout);

    let utxo_amount =
        OmniAmount::from_sat((first_unspent["amount"].as_f64().unwrap() * 100_000_000.0) as u64);
    println!("UTXO amount: {:?}", utxo_amount);

    let change_amount: OmniAmount = utxo_amount - OMNI_SPEND_AMOUNT - OmniAmount::from_sat(1000); // 1000 satoshis for fee
    println!("change_amount: {:?}", change_amount);

    let change_txout = OmniTxOut {
        value: change_amount,
        script_pubkey: OmniScriptBuf(bob_script_pubkey.as_bytes().to_vec()),
    };

    println!("change_txout: {:?}", change_txout);

    let mut omni_tx: BitcoinTransaction = TransactionBuilder::new::<BITCOIN>()
        .version(OmniVersion::One)
        .lock_time(OmniLockTime::from_height(1).unwrap())
        .inputs(vec![txin])
        .outputs(vec![txout, change_txout])
        .build();

    println!("tx: {:?}", omni_tx.clone());

    let omni_tx_encoded = omni_tx.clone().build_for_signing();
    println!("omni_tx_encoded: {:?}", omni_tx_encoded);

    let sighash_type = EcdsaSighashType::All;
    let mut omni_tx_encoded = omni_tx_encoded.clone();
    omni_tx_encoded.extend_from_slice(&sighash_type.to_u32().to_le_bytes());

    let sighash = sha256d::Hash::hash(&omni_tx_encoded);
    println!("sighash: {:?}", sighash);

    // Create a Message from the sighash
    let msg = Message::from_digest_slice(&sighash.to_byte_array()).unwrap();
    println!("msg: {:?}", msg);

    // Sign the message
    let signature = secp.sign_ecdsa(&msg, &first_priv_key);
    println!("signature: {:?}", signature);

    // Verify signature
    let is_valid = secp.verify_ecdsa(&msg, &signature, &public_key).is_ok();
    println!("Signature valid: {:?}", is_valid);

    assert!(is_valid, "The signature should be valid");

    let signature = bitcoin::ecdsa::Signature {
        signature,
        sighash_type,
    };

    // Create the script_sig
    let script_sig: bitcoin::ScriptBuf = Builder::new()
        .push_slice(&signature.serialize())
        .push_key(&bitcoin_public_key)
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
            bitcoin_public_key.to_bytes(),
            "Public key mismatch in script_sig"
        );
    } else {
        panic!("Expected public key push in script_sig");
    }

    // Ensure there are no more instructions
    assert!(iter.next().is_none(), "Unexpected data in script_sig");

    println!("script_sig verification passed");

    // Assign script_sig to txin
    let omni_script_sig = OmniScriptBuf(script_sig.as_bytes().to_vec());
    println!("omni_script_sig: {:?}", omni_script_sig);
    omni_tx.input[0].script_sig = omni_script_sig;

    // Finalize the transaction
    let tx_signed = omni_tx;

    println!("tx_signed: {:?}", tx_signed);

    // Serialize the transaction
    let new_tx = tx_signed.clone().build_for_signing();
    let new_tx_hex = hex::encode(new_tx.clone());

    println!("Transaction hex: {}", new_tx_hex);

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
        script_pubkey: alice_script_pubkey.clone(),
    };

    let utxo_amount_btc = Amount::from_btc(first_unspent["amount"].as_f64().unwrap())?;
    let change_amount: Amount = utxo_amount_btc - SPEND_AMOUNT - Amount::from_sat(1000); // 1000 satoshis for fee

    let change_txout_btc = TxOut {
        value: change_amount,
        script_pubkey: bob_script_pubkey.clone(),
    };

    let mut tx_btc = Transaction {
        version: transaction::Version::ONE,
        lock_time: LockTime::Blocks(Height::from_consensus(1).unwrap()),
        input: vec![txin_btc],
        output: vec![txout_btc, change_txout_btc],
    };

    // Serializa la transacción de Bitcoin
    let tx_btc_hex = hex::encode(bitcoin::consensus::encode::serialize(&tx_btc));
    println!("Bitcoin Transaction hex: {}", tx_btc_hex);

    // Get the sighash to sign.
    let sighash_type = EcdsaSighashType::All;
    let sighasher = SighashCache::new(&mut tx_btc);
    let sighash_btc = sighasher
        .legacy_signature_hash(0, &bob_script_pubkey, sighash_type.to_u32())
        .expect("failed to create sighash");

    println!("sighash btc: {:?}", sighash_btc);

    let msg_btc = Message::from(sighash_btc);
    println!("msg btc: {:?}", msg_btc);

    let signature_btc = secp.sign_ecdsa(&msg_btc, &first_priv_key);
    println!("signature btc: {:?}", signature_btc);

    // Verify signature
    let is_valid = secp.verify_ecdsa(&msg, &signature_btc, &public_key).is_ok();
    println!("Signature valid: {:?}", is_valid);

    assert!(is_valid, "The signature should be valid");

    let signature = bitcoin::ecdsa::Signature {
        signature: signature_btc,
        sighash_type,
    };

    // Create the script_sig
    let script_sig_btc = Builder::new()
        .push_slice(&signature.serialize())
        .push_key(&bitcoin_public_key)
        .into_script();

    // Assign script_sig to txin
    tx_btc.input[0].script_sig = script_sig_btc;

    // Finalize the transaction
    let tx_signed_btc = tx_btc;

    println!("tx_signed_btc: {:?}", tx_signed_btc);

    let tx_signed_btc_hex = hex::encode(bitcoin::consensus::encode::serialize(&tx_signed_btc));
    println!("tx_signed_btc_hex: {}", tx_signed_btc_hex);

    assert_eq!(new_tx_hex, tx_signed_btc_hex);

    // -------------------------------------------------------------------------------------------------
    // Try to send the raw transaction
    // let raw_tx_result: Result<serde_json::Value, _> =
    //     client.call("sendrawtransaction", &[json!(new_tx_hex)]);

    // 0100000001fd5bab205d41bab5190697ee18ce0c288522affefe5cecb1aebc76a5d187f4aa000000006a473044022068fbb558a2292773aa65707cb2d6e6d61182d0322d551c6f0203d86d0479e19e02202dea83603a8042ea4214da7fe3e9d45db96ba6c2145b9bd07f9c872fa603240f012102c5ff30877255f0dba3c02b654c4cd91e9a6040cf8eee365b8e4f728286a919ceffffffff020065cd1d000000001976a91425eacbe1f213f32b0944fc27a15ecd1eeba425d588ac1889380c010000001976a91401612abcbbb29e4bdb842262d78b6ce78ba5c39288ac01000000
    // 010000000100792b0147643f4fcc462fdde47f68199e924f0c24ff89ea245c1805e961d109000000006b483045022100fcda0512f9a26dd27a27f14f69033ea6eaf89b95ccd02c24139ef66f3280939702203222b0f0e4eeca034b41b14b989b27593dd9ed01903aa36977607a56c7f834d7012103d6adde1890718b115950f5bca8e4c812c458a299248665362dd95aa9f6bf7184ffffffff020065cd1d000000001976a91426cb36e2f905129f72936bf468f072e3153f5b6d88ac1889380c010000001976a91445b3d4176313065e6808d3b4f2816764c8e981cc88ac01000000
    // match raw_tx_result {
    //     Ok(result) => println!("Transaction sent successfully: {:?}", result),
    //     Err(e) => {
    //         println!("Failed to send transaction: {:?}", e);

    //         // // Check if the error is due to missing inputs
    //         // if e.to_string().contains("bad-txns-inputs-missingorspent") {
    //         //     // Verify UTXO
    //         //     let txid = tx_signed.input[0].previous_output.txid.0.as_byte_array();
    //         //     let vout = tx_signed.input[0].previous_output.vout;
    //         //     let utxo_info: Option<serde_json::Value> =
    //         //         client.call("gettxout", &[json!(txid), json!(vout)])?;
    //         //     println!("UTXO info: {:?}", utxo_info);
    //         // }
    //     }
    // }

    Ok(())
}

// BUENA
// sighash 30e4882aadacc2899ddcae75d2959c6dfed31487c77bb0afc19087646807cac2
// msg: Message(30e4882aadacc2899ddcae75d2959c6dfed31487c77bb0afc19087646807cac2)
// signature: 304402207c973eb79a803808f67c384056056ce340a8b0dbc1c5b50f13f1567bbcea27c3022007852da341f4455ac492de1ae4957093c44d4559b6140d07d4fb2c2468e5eda7

// OMNI ONE?
// sighash 0x8e12f6a2caa3fae8e6c48e399c6be5f0c9fe58342cb31e68d735fb878754209a
// message 9a20548787fb35d7681eb32c3458fec9f0e56b9c398ec4e6e8faa3caa2f6128e
// signature 304502210080916d2b2f2db05ec33e12eaed1d7a3e2165578a40b1dc8116623cd6473939a802201a02bb40cd34068556756f36e2eb4df87c897f2aaba95c558b475d0ad50d8294
// #[tokio::test]
// #[ignore]
// async fn test_send_p2pkh_using_rust_bitcoin() -> Result<()> {
//     let bitcoind = bitcoind::BitcoinD::from_downloaded().unwrap();
//     let client: &bitcoind::Client = &bitcoind.client;

//     // Setup Bob and Alice addresses
//     let (bob_address, bob_script_pubkey) =
//         get_address_info_for(client, "Bob").expect("Failed to get address info for Bob");

//     let (alice_address, alice_script_pubkey) =
//         get_address_info_for(client, "Alice").expect("Failed to get address info for Alice");

//     // Get descriptors
//     let master_key = get_master_key_of_regtest_node(client).expect("Failed to get master key");

//     // Initialize secp256k1 context
//     let secp = Secp256k1::new();

//     // Derive child private key using path m/44h/1h/0h
//     let path = "m/44h/1h/0h".parse::<DerivationPath>().unwrap();
//     let child = master_key.derive_priv(&secp, &path).unwrap();
//     println!("Child at {}: {}", path, child);

//     let xpub = Xpub::from_priv(&secp, &child);
//     println!("Public key at {}: {}", path, xpub);

//     // Generate first P2PKH address at m/0/0
//     let zero = ChildNumber::Normal { index: 0 };
//     let public_key = xpub.derive_pub(&secp, &[zero, zero]).unwrap().public_key;
//     let bitcoin_public_key = bitcoin::PublicKey::new(public_key);
//     let derived_bob_address = Address::p2pkh(&bitcoin_public_key, Network::Regtest);

//     assert_eq!(bob_address, derived_bob_address);

//     // Get private key for first P2PKH address
//     let first_priv_key = child
//         .derive_priv(&secp, &DerivationPath::from(vec![zero, zero]))
//         .unwrap()
//         .private_key;

//     println!(
//         "Private key for first receiving address: {:?}",
//         first_priv_key
//     );

//     // Generate 101 blocks to the address
//     client.generate_to_address(101, &bob_address)?;

//     // List UTXOs for Bob
//     let min_conf = 1;
//     let max_conf = 9999999;
//     let include_unsafe = true;
//     let query_options = json!({});
//     let unspent_utxos_bob: Vec<serde_json::Value> = client.call(
//         "listunspent",
//         &[
//             json!(min_conf),
//             json!(max_conf),
//             json!(vec![bob_address.to_string()]),
//             json!(include_unsafe),
//             query_options.clone(),
//         ],
//     )?;

//     println!("UTXOs for Bob: {:?}", unspent_utxos_bob);

//     // Get the first UTXO
//     let first_unspent = unspent_utxos_bob
//         .into_iter()
//         .next()
//         .expect("There should be at least one unspent output");

//     println!("First UTXO: {:?}", first_unspent);

//     // Verify UTXO belongs to our address and has the correct amount
//     assert_eq!(
//         first_unspent["address"].as_str().unwrap(),
//         bob_address.to_string(),
//         "UTXO doesn't belong to Bob"
//     );

//     let utxo_amount = Amount::from_btc(first_unspent["amount"].as_f64().unwrap())?;
//     println!("UTXO amount: {:?}", utxo_amount);

//     // Check if the UTXO amount is 50 BTC
//     assert_eq!(
//         utxo_amount.to_sat(),
//         5_000_000_000,
//         "UTXO amount is not 50 BTC"
//     );

//     // Generate second (alice) P2PKH address at m/0/1
//     let one = ChildNumber::Normal { index: 1 };
//     let alice_public_key = xpub.derive_pub(&secp, &[zero, one]).unwrap().public_key;
//     let alice_bitcoin_public_key = bitcoin::PublicKey::new(alice_public_key);
//     let derived_alice_address = Address::p2pkh(&alice_bitcoin_public_key, Network::Regtest);
//     println!("Alice derived address: {}", derived_alice_address);

//     assert_eq!(alice_address, derived_alice_address);

//     let txin = TxIn {
//         previous_output: OutPoint::new(
//             first_unspent["txid"].as_str().unwrap().parse()?,
//             first_unspent["vout"].as_u64().unwrap() as u32,
//         ),
//         script_sig: ScriptBuf::new(), // Initially empty, will be filled later with the signature
//         sequence: Sequence::MAX,
//         witness: Witness::default(),
//     };

//     let txout = TxOut {
//         value: SPEND_AMOUNT,
//         script_pubkey: alice_script_pubkey.clone(),
//     };

//     println!("txout: {:?}", txout);

//     let change_amount: Amount = utxo_amount - SPEND_AMOUNT - Amount::from_sat(1000); // 1000 satoshis for fee
//     println!("change_amount: {:?}", change_amount);

//     let change_txout = TxOut {
//         value: change_amount,
//         script_pubkey: bob_script_pubkey.clone(),
//     };

//     println!("change_txout: {:?}", change_txout);

//     let mut tx = Transaction {
//         version: transaction::Version::ONE,
//         lock_time: LockTime::Blocks(Height::from_consensus(1).unwrap()),
//         input: vec![txin],
//         output: vec![txout, change_txout],
//     };
//     println!("tx: {:?}", tx.clone());

//     // Get the sighash to sign.
//     let sighash_type = EcdsaSighashType::All;
//     let sighasher = SighashCache::new(&mut tx);
//     let sighash = sighasher
//         .legacy_signature_hash(0, &bob_script_pubkey, sighash_type.to_u32())
//         .expect("failed to create sighash");

//     println!("sighash: {:?}", sighash);

//     let msg = Message::from(sighash);
//     println!("msg: {:?}", msg);

//     let signature = secp.sign_ecdsa(&msg, &first_priv_key);
//     println!("signature: {:?}", signature);

//     // Verify signature
//     let is_valid = secp.verify_ecdsa(&msg, &signature, &public_key).is_ok();
//     println!("Signature valid: {:?}", is_valid);

//     assert!(is_valid, "The signature should be valid");

//     let signature = bitcoin::ecdsa::Signature {
//         signature,
//         sighash_type,
//     };

//     // Create the script_sig
//     let script_sig = Builder::new()
//         .push_slice(&signature.serialize())
//         .push_key(&bitcoin_public_key)
//         .into_script();

//     // Verify the script_sig
//     println!("script_sig: {:?}", script_sig);

//     // Decode the script_sig to verify its contents
//     let mut iter = script_sig.instructions().peekable();

//     // Check the signature
//     if let Some(Ok(Instruction::PushBytes(sig_bytes))) = iter.next() {
//         println!("Signature in script_sig: {:?}", sig_bytes);

//         assert_eq!(
//             sig_bytes.as_bytes(),
//             signature.serialize().to_vec().as_slice(),
//             "Signature mismatch in script_sig"
//         );
//     } else {
//         panic!("Expected signature push in script_sig");
//     }

//     // Check the public key
//     if let Some(Ok(Instruction::PushBytes(pubkey_bytes))) = iter.next() {
//         println!("Public key in script_sig: {:?}", pubkey_bytes);
//         assert_eq!(
//             pubkey_bytes.as_bytes(),
//             bitcoin_public_key.to_bytes(),
//             "Public key mismatch in script_sig"
//         );
//     } else {
//         panic!("Expected public key push in script_sig");
//     }

//     // Ensure there are no more instructions
//     assert!(iter.next().is_none(), "Unexpected data in script_sig");

//     println!("script_sig verification passed");

//     // Assign script_sig to txin
//     tx.input[0].script_sig = script_sig;

//     // Finalize the transaction
//     let tx_signed = tx;

//     println!("tx_signed: {:?}", tx_signed);

//     let raw_tx_result = client.send_raw_transaction(&tx_signed).unwrap();
//     println!("raw_tx_result: {:?}", raw_tx_result);

//     client.generate_to_address(101, &bob_address)?;

//     assert_utxos_for_address(client, alice_address, 1);

//     Ok(())
// }

// fn assert_utxos_for_address(client: &bitcoind::Client, address: Address, number_of_utxos: usize) {
//     let min_conf = 1;
//     let max_conf = 9999999;
//     let include_unsafe = true;
//     let query_options = json!({});

//     let unspent_utxos: Vec<serde_json::Value> = client
//         .call(
//             "listunspent",
//             &[
//                 json!(min_conf),
//                 json!(max_conf),
//                 json!(vec![address.to_string()]),
//                 json!(include_unsafe),
//                 query_options.clone(),
//             ],
//         )
//         .unwrap();

//     assert!(
//         unspent_utxos.len() == number_of_utxos,
//         "Expected {} UTXOs for address {}, but found {}",
//         number_of_utxos,
//         address.to_string(),
//         unspent_utxos.len()
//     );
// }
