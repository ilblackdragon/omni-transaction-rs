// Rust Bitcoin
use bitcoin::hashes::{sha256d, Hash};
use bitcoin::script::Builder;
use bitcoin::secp256k1::{Message, Secp256k1};
use bitcoin::Address;
use bitcoin::EcdsaSighashType;
// Omni library
use omni_transaction::bitcoin::bitcoin_transaction::BitcoinTransaction;
use omni_transaction::bitcoin::types::{
    Amount as OmniAmount, EcdsaSighashType as OmniSighashType, Hash as OmniHash,
    LockTime as OmniLockTime, OutPoint as OmniOutPoint, ScriptBuf as OmniScriptBuf,
    Sequence as OmniSequence, TransactionType, TxIn as OmniTxIn, TxOut as OmniTxOut,
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
    let sighash_type = OmniSighashType::All;
    let encoded_data = omni_tx.build_for_signing(sighash_type);

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

    println!("raw_tx_result: {:?}", raw_tx_result);

    client.generate_to_address(101, &bob.address)?;

    assert_utxos_for_address(client, alice.address, 1);

    Ok(())
}

#[tokio::test]
async fn test_send_p2wpkh_using_rust_bitcoin_and_omni_library() -> Result<()> {
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
    let sighash_type = OmniSighashType::All;
    let encoded_data = omni_tx.build_for_signing(sighash_type);

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
                query_options,
            ],
        )
        .unwrap();

    assert!(
        unspent_utxos.len() == number_of_utxos,
        "Expected {} UTXOs for address {}, but found {}",
        number_of_utxos,
        address,
        unspent_utxos.len()
    );
}

// Tipos de Transacciones y SegWit
// 1. P2PKH (Pay-to-PubKey-Hash): La transacción estándar donde el script_sig contiene la firma y la clave pública.
// 2. P2SH (Pay-to-Script-Hash): La transacción donde el script_sig contiene el script de redención.
// 3. P2WPKH (Pay-to-Witness-PubKey-Hash): La transacción SegWit donde la firma y la clave pública están en la parte de testigo (witness).
// 4. P2WSH (Pay-to-Witness-Script-Hash): La transacción SegWit donde el script de redención está en la parte de testigo (witness).
// Cómo Afecta el EcdsaSighashType
// El EcdsaSighashType afecta qué partes de la transacción se incluyen en el cálculo del sighash. Aquí hay un resumen:
// SIGHASH_ALL: Firma todos los inputs y outputs.
// SIGHASH_NONE: Firma solo los inputs, excluyendo todos los outputs.
// SIGHASH_SINGLE: Firma solo el input correspondiente y el output con el mismo índice.
// SIGHASH_ANYONECANPAY: Permite que otros inputs sean añadidos a la transacción sin invalidar la firma.

//     pub fn build_and_sign(&self, private_keys: &[bitcoin::PrivateKey], tx_type: &str) -> BitcoinTransaction {
//         let mut tx = self.build();

//         let sighash_type = self.sighash_type.expect("Missing sighash type");

//         for (i, input) in tx.input.iter_mut().enumerate() {
//             // Extend the transaction with the sighash type
//             let mut encoded_data = tx.build_for_signing(i, sighash_type);
//             let sighash_type_bytes = sighash_type.to_u32().to_le_bytes();
//             encoded_data.extend_from_slice(&sighash_type_bytes);

//             // Calculate the sighash
//             let sighash = sha256d::Hash::hash(&encoded_data);
//             let msg = Message::from_digest_slice(sighash.as_byte_array()).unwrap();

//             // Sign the sighash
//             let secp = Secp256k1::new();
//             let signature = secp.sign_ecdsa(&msg, &private_keys[i].key);

//             // Create the script_sig or witness based on tx_type
//             match tx_type {
//                 "P2PKH" => {
//                     let script_sig = Builder::new()
//                         .push_slice(&signature.serialize())
//                         .push_key(&private_keys[i].public_key(&secp))
//                         .into_script();
//                     input.script_sig = script_sig;
//                 }
//                 "P2SH" => {
//                     // Assuming redeem_script is provided
//                     let redeem_script = ...; // Define your redeem script
//                     let script_sig = Builder::new()
//                         .push_slice(&signature.serialize())
//                         .push_key(&private_keys[i].public_key(&secp))
//                         .push_slice(&redeem_script)
//                         .into_script();
//                     input.script_sig = script_sig;
//                 }
//                 "P2WPKH" => {
//                     let witness = vec![
//                         signature.serialize().to_vec(),
//                         private_keys[i].public_key(&secp).to_bytes(),
//                     ];
//                     input.witness = witness;
//                 }
//                 "P2WSH" => {
//                     // Assuming witness_script is provided
//                     let witness_script = ...; // Define your witness script
//                     let witness = vec![
//                         signature.serialize().to_vec(),
//                         witness_script.to_bytes(),
//                     ];
//                     input.witness = witness;
//                 }
//                 _ => panic!("Unsupported transaction type"),
//             }
//         }

//         tx
//     }
// }
