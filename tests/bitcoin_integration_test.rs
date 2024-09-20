use bitcoin::absolute::Height;
use bitcoin::bip32::{ChildNumber, DerivationPath, Xpriv, Xpub};
// use bitcoin::address::script_pubkey::ScriptBufExt as _;
use bitcoin::hashes::Hash;
use bitcoin::locktime::absolute;
// use bitcoin::script::PushBytes;
// use bitcoin::script::Builder;
use bitcoin::secp256k1::{rand, Message, Secp256k1, SecretKey, Signing};
use bitcoin::sighash::{EcdsaSighashType, SighashCache};
use bitcoin::{
    transaction, Address, Amount, Network, OutPoint, ScriptBuf, Sequence, Transaction, TxIn, TxOut,
    Txid, WPubkeyHash, Witness,
};
use bitcoind::AddressType;
use bitcoind::Conf;

use eyre::Result;
use serde_json::{json, Value};
use std::result::Result::Ok;
use std::str::FromStr;

const SPEND_AMOUNT: Amount = Amount::from_sat(5_000_000);

mod utils;

pub use utils::bitcoin_utils::*;

#[tokio::test]
async fn test_send_p2pkh() -> Result<()> {
    let conf = Conf::default();
    println!("Configuration params for bitcoind: {:?}", conf);

    let bitcoind = bitcoind::BitcoinD::from_downloaded_with_conf(&conf).unwrap();
    let client: &bitcoind::Client = &bitcoind.client;

    let (alice_address, alice_script_pubkey) =
        get_address_info_for(client, "Alice").expect("Failed to get address info for Alice");

    let (bob_address, bob_script_pubkey) =
        get_address_info_for(client, "Bob").expect("Failed to get address info for Bob");

    // Get descriptors
    let descriptors: Value = client.call("listdescriptors", &[true.into()])?;

    let p2pkh_descriptor = descriptors["descriptors"]
        .as_array()
        .unwrap()
        .iter()
        .find(|descriptor| descriptor["desc"].as_str().unwrap().contains("pkh"))
        .expect("No P2PKH descriptor found");

    println!("p2pkh_descriptor: {:?}", p2pkh_descriptor);

    let desc = p2pkh_descriptor["desc"].as_str().unwrap();
    let parts: Vec<&str> = desc.split('/').collect();
    let master_key_str = parts[0].replace("pkh(", "").replace(")", "");
    println!("master_key_str: {:?}", master_key_str);

    // Initialize secp256k1 context
    let secp = Secp256k1::new();

    // Derive master private key
    let master_key = Xpriv::from_str(&master_key_str).unwrap();
    println!("Master key: {}", master_key);

    // Derive child private key using path m/44h/1h/0h
    let path = "m/44h/1h/0h".parse::<DerivationPath>().unwrap();
    let child = master_key.derive_priv(&secp, &path).unwrap();
    println!("Child at {}: {}", path, child);

    let xpub = Xpub::from_priv(&secp, &child);
    println!("Public key at {}: {}", path, xpub);

    // Generate first P2PKH address at m/0/0
    let zero = ChildNumber::Normal { index: 0 };
    let public_key = xpub.derive_pub(&secp, &[zero, zero]).unwrap().public_key;
    let bitcoin_public_key = bitcoin::PublicKey::new(public_key);
    let derived_bob_address = Address::p2pkh(&bitcoin_public_key, Network::Regtest);
    println!("First receiving address: {}", derived_bob_address);

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

    // Listar UTXOs para Bob
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
    println!("UTXOs disponibles para Bob: {:?}", unspent_utxos_bob);

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

    let utxo_amount = Amount::from_btc(first_unspent["amount"].as_f64().unwrap())?;
    println!("UTXO amount: {:?}", utxo_amount);

    // Check if the UTXO amount is 50 BTC
    assert_eq!(
        utxo_amount.to_sat(),
        5_000_000_000,
        "UTXO amount is not 50 BTC"
    );

    // Get address for Alice
    let alice_address = client
        .get_new_address_with_type(AddressType::Legacy)
        .unwrap()
        .address()
        .unwrap();

    let alice_address = alice_address.require_network(Network::Regtest).unwrap();
    println!("Alice address: {:?}", alice_address);

    // Generate second (alice) P2PKH address at m/0/1
    let one = ChildNumber::Normal { index: 1 };
    let alice_public_key = xpub.derive_pub(&secp, &[zero, one]).unwrap().public_key;
    let alice_bitcoin_public_key = bitcoin::PublicKey::new(alice_public_key);
    let derived_alice_address = Address::p2pkh(&alice_bitcoin_public_key, Network::Regtest);
    println!("Alice derived address: {}", derived_alice_address);

    assert_eq!(alice_address, derived_alice_address);

    // Create the script_pubkey for Alice
    let alice_script_pubkey = ScriptBuf::from_hex(&alice_address.to_string())
        .expect("unable to convert address to scriptbuf");

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
        script_pubkey: alice_script_pubkey.clone(),
    };

    println!("txout: {:?}", txout);

    let change_amount: Amount = utxo_amount - SPEND_AMOUNT - Amount::from_sat(1000); // 1000 satoshis for fee
    println!("change_amount: {:?}", change_amount);

    let change_txout = TxOut {
        value: change_amount,
        script_pubkey: bob_script_pubkey.clone(),
    };

    println!("change_txout: {:?}", change_txout);

    let tx = Transaction {
        version: transaction::Version::ONE,
        lock_time: absolute::LockTime::Blocks(Height::from_consensus(1).unwrap()),
        input: vec![txin],
        output: vec![txout, change_txout],
    };
    println!("tx: {:?}", tx);

    // Get the sighash to sign.
    let sighash_type = EcdsaSighashType::All;
    let sighasher = SighashCache::new(tx.clone());
    let sighash = sighasher
        .legacy_signature_hash(0, &alice_script_pubkey, sighash_type.to_u32())
        .expect("failed to create sighash");

    println!("sighash: {:?}", sighash);

    // let msg = Message::from(sighash);
    // println!("msg: {:?}", msg);

    // let signed_tx_via_manual_message = client.sign_message(msg, &alice_address)?;
    // println!(
    //     "signed_tx_via_manual_message: {:?}",
    //     signed_tx_via_manual_message
    // );

    // let txid = client.call::<String>("sendrawtransaction", &[signed_tx_hex.into()])?;
    // println!("Transaction sent. TxID: {:?}", txid);

    // let signed_tx_result_via_raw_tx = client.send_raw_transaction(&signed_tx).unwrap();
    // println!(
    //     "signed_tx_result_via_raw_tx: {:?}",
    //     signed_tx_result_via_raw_tx
    // );

    // Ahora firma la transacción
    //    let signed_tx_result: Value = client.call("signrawtransactionwithwallet", &[unsigned_tx_hex.into()])?;

    // Después de crear tu transacción sin firmar

    // Serializa la transacción sin firmar a formato hexadecimal
    // let unsigned_tx_hex = bitcoin::consensus::encode::serialize_hex(&tx);

    // // Usa el método RPC signrawtransactionwithwallet para firmar la transacción
    // let signed_tx_result: Value = client.call("signrawtransactionwithwallet", &[unsigned_tx_hex.into()])?;

    // // Extrae la transacción firmada del resultado
    // let signed_tx_hex = signed_tx_result["hex"].as_str().expect("No signed transaction hex found");

    // // Deserializa la transacción firmada de vuelta a un objeto Transaction
    // let signed_tx: Transaction = bitcoin::consensus::encode::deserialize(&hex::decode(signed_tx_hex)?)?;

    // println!("Signed transaction: {:?}", signed_tx);

    // // Ahora puedes enviar esta transacción firmada

    // let signature = secp.sign_ecdsa(&msg, &sk);
    // let signature = secp.sign_ecdsa(&msg, &sk);

    // println!("signature: {:?}", signature);

    // let signature = bitcoin::ecdsa::Signature {
    //     signature,
    //     sighash_type,
    // };

    // println!("signature: {:?}", signature);
    // // Create the script_sig
    // let script_sig = Builder::new()
    //     .push_slice(&signature.serialize())
    //     .push_key(&pk)
    //     .into_script();

    // println!("script_sig: {:?}", script_sig);

    // // Asignar script_sig a txin
    // tx.input[0].script_sig = script_sig;

    // println!("tx: {:?}", tx);

    // // Get the signed transaction
    // // let tx_signed = sighasher.into_transaction();

    // // println!("tx_signed: {:?}", tx_signed);

    // let raw_tx_result = client.send_raw_transaction(&tx).unwrap();

    // println!("raw_tx_result: {:?}", raw_tx_result);

    Ok(())
}

// #[tokio::test]
// async fn test_send_p2wpkh() -> Result<()> {
//     let bitcoind = BitcoinD::from_downloaded().unwrap();
//     let client = &bitcoind.client;

//     // Generar una nueva dirección SegWit
//     let address = client.call::<String>("getnewaddress", &[json!("bech32")])?;
//     println!("Nueva dirección SegWit: {}", address);

//     // Generar algunos bloques para obtener monedas
//     client.call::<Vec<String>>("generatetoaddress", &[json!(101), json!(address)])?;

//     // Listar UTXOs
//     let unspent: Vec<serde_json::Value> = client.call("listunspent", &[])?;
//     println!("UTXOs disponibles: {:?}", unspent);

//     // Obtener el primer UTXO
//     let first_unspent = unspent.into_iter().next().expect("There should be at least one unspent output");
//     println!("Primer UTXO: {:?}", first_unspent);

//     // Crear y firmar la transacción P2WPKH
//     // Aquí deberías agregar el código para crear y firmar la transacción P2WPKH

//     Ok(())
// }
