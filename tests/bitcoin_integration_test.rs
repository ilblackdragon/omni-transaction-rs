use bitcoin::absolute::Height;
use bitcoin::bip32::{ChildNumber, DerivationPath, Xpriv, Xpub};
// use bitcoin::address::script_pubkey::ScriptBufExt as _;
// use bitcoin::blockdata::opcodes::all::{OP_CHECKSIG, OP_DUP, OP_EQUALVERIFY, OP_HASH160};
// use bitcoin::blockdata::script::Builder;
use bitcoin::hashes::Hash;
use bitcoin::locktime::absolute;

// use bitcoin::script::PushBytes;
// use bitcoin::script::Builder;
use bitcoin::secp256k1::{rand, Message, Secp256k1, SecretKey, Signing};
use bitcoin::sighash::{EcdsaSighashType, SighashCache};
use bitcoin::{
    transaction, Address, Amount, Network, NetworkKind, OutPoint, PrivateKey, ScriptBuf, Sequence,
    Transaction, TxIn, TxOut, Txid, WPubkeyHash, Witness,
};
use bitcoind::AddressType;
use bitcoind::Conf;

use eyre::Result;
use serde_json::{json, Value};
use std::result::Result::Ok;
use std::str::FromStr;

const DUMMY_UTXO_AMOUNT: Amount = Amount::from_sat(20_000_000);
// const SPEND_AMOUNT: Amount = Amount::from_sat(5_000_000);
const CHANGE_AMOUNT: Amount = Amount::from_sat(14_999_000); // 1000 sat fee.

const SPEND_AMOUNT: Amount = Amount::from_sat(5_000_000);
// const CHANGE_AMOUNT: Amount = Amount::from_sat(14_999_000); // 1000 sat fee.

// #[tokio::test]
// async fn hello_bitcoin() -> Result<()> {
//     let bitcoind = bitcoind::BitcoinD::from_downloaded().unwrap();
//     let info = bitcoind.client.get_blockchain_info().unwrap();

//     println!("{:?}", info);

//     assert_eq!(0, info.blocks);
//     assert_eq!(info.chain, "regtest");
//     assert_eq!(info.blocks, 0);

//     Ok(())
// }

// AYUDA
// let bitcoind = bitcoind::BitcoinD::from_downloaded().unwrap();

// // Generate some blocks to get coins
// bitcoind.client.generate_to_address(101, &bitcoind.client.get_new_address(None, None)?)?;

// // Get a real UTXO
// let unspent = bitcoind.client.list_unspent(None, None, None, None, None)?
//     .into_iter()
//     .next()
//     .expect("There should be at least one unspent output");

// let secp = Secp256k1::new();
// let (sk, _) = senders_keys(&secp);

// // Use the real UTXO details
// let dummy_out_point = OutPoint::new(unspent.txid, unspent.vout);
// let dummy_utxo = TxOut {
//     value: Amount::from_btc(unspent.amount)?,

// TESTS
// P2PKH
// P2WPKH

// fn create_wallet_for_bob(bitcoind: &bitcoind::BitcoinD, name: &str) -> (Address, ScriptBuf) {
//     // Create a new address for the recipient
//     let bob = bitcoind.create_wallet("bob").unwrap();
//     let bob_address = bob
//         .get_new_address_with_type(AddressType::Legacy)
//         .unwrap()
//         .address()
//         .unwrap();

//     let bob_address = bob_address.require_network(Network::Regtest).unwrap();
//     println!("Bob address: {:?}", bob_address);

//     // Get address info for Bob
//     let bob_address_info: Value = bob.call("getaddressinfo", &[bob_address.to_string().into()])?;
//     println!("Bob Address info: {:?}", bob_address_info);

//     // Extract the scriptPubKey from the result
//     let bob_script_pubkey_hex = bob_address_info["scriptPubKey"]
//         .as_str()
//         .expect("scriptPubKey should be a string");

//     let bob_script_pubkey =
//         ScriptBuf::from_hex(bob_script_pubkey_hex).expect("Failed to parse scriptPubKey");

//     println!("Bob ScriptPubKey: {:?}", bob_script_pubkey);

//     println!("Bob address: {:?}", bob_address);

//     return (bob_address, bob_script_pubkey);
// }

// fn create_wallet_for_alice(bitcoind: &bitcoind::BitcoinD, name: &str) -> (Address, ScriptBuf) {
//     // Create a new address for the recipient
//     let alice = bitcoind.create_wallet("alice").unwrap();
//     let alice_address = alice
//         .get_new_address_with_type(AddressType::Legacy)
//         .unwrap()
//         .address()
//         .unwrap();

//     let alice_address = alice_address.require_network(network).unwrap();
//     println!("New address: {:?}", alice_address);

//     // Get address info for Alice
//     let alice_address_info: Value =
//         alice.call("getaddressinfo", &[alice_address.to_string().into()])?;
//     println!("Address info: {:?}", alice_address_info);

//     // Extract the scriptPubKey from the result
//     let alice_script_pubkey_hex = alice_address_info["scriptPubKey"]
//         .as_str()
//         .expect("scriptPubKey should be a string");

//     let alice_script_pubkey =
//         ScriptBuf::from_hex(alice_script_pubkey_hex).expect("Failed to parse scriptPubKey");

//     println!("Alice ScriptPubKey: {:?}", alice_script_pubkey);

//     let alice_pubkey = alice_address_info["pubkey"]
//         .as_str()
//         .expect("pubkey should be a string");
//     println!("Alice Pubkey: {:?}", alice_pubkey);

//     let alice_pubkey_bytes = hex::decode(alice_pubkey).unwrap();
//     let alice_pubkey = bitcoin::PublicKey::from_slice(&alice_pubkey_bytes).unwrap();
//     let alice_pubkey_hash = alice_pubkey.pubkey_hash();
//     println!("Alice Pubkey Hash: {:?}", alice_pubkey_hash);

//     return (alice_address, alice_script_pubkey);
// }

#[tokio::test]
async fn test_send_p2pkh() -> Result<()> {
    let conf = Conf::default();
    println!("Configuration params for bitcoind: {:?}", conf);

    let bitcoind = bitcoind::BitcoinD::from_downloaded_with_conf(&conf).unwrap();
    let client = &bitcoind.client;

    let bob_address = client
        .get_new_address_with_type(AddressType::Legacy)
        .unwrap()
        .address()
        .unwrap();

    let bob_address = bob_address.require_network(Network::Regtest).unwrap();
    println!("Bob address: {:?}", bob_address);

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
    let address = Address::p2pkh(&bitcoin_public_key, Network::Regtest);
    println!("First receiving address: {}", address);

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

    // let (bob_address, bob_script_pubkey) = create_wallet_for_bob(&bitcoind, "bob");
    // let (alice_address, alice_script_pubkey) = create_wallet_for_alice(&bitcoind, "alice");

    // let secp = Secp256k1::new();
    // let sk = SecretKey::new(&mut rand::thread_rng());
    // let pk = bitcoin::PublicKey::new(sk.public_key(&secp));
    // let hetman_address = Address::p2pkh(&pk, Network::Regtest);
    // println!("Hetman address: {:?}", hetman_address);

    // let create_wallet_result: Value = client.call(
    //     "createwallet",
    //     &[
    //         "testwallet".into(),
    //         false.into(),
    //         false.into(),
    //         "".into(),
    //         false.into(),
    //         false.into(),
    //     ],
    // )?;
    // println!("Create wallet result: {:?}", create_wallet_result);

    // // Verificar si la cartera es legacy
    // let wallet_info: Value = client.call("getwalletinfo", &[])?;
    // println!("Wallet info: {:?}", wallet_info);

    // let descriptors: Value = client.call("listdescriptors", &[true.into()])?;
    // println!("Descriptors: {:?}", descriptors);

    // let new_address: Value = client.call("getnewaddress", &[json!("legacy")])?;

    // println!("New address: {:?}", new_address);

    // let dump_result: Value = client.call("dumpprivkey", &[new_address.into()])?;

    // println!("Dump privkey result: {:?}", dump_result);

    // let wif = PrivateKey {
    //     compressed: true,
    //     network: NetworkKind::Test,
    //     inner: sk,
    // }
    // .to_wif();

    // let import_result: Value =
    //     client.call("importprivkey", &[wif.into(), "".into(), false.into()])?;

    // println!("Import privkey result: {:?}", import_result);

    // Generate 101 blocks to the address
    // client.generate_to_address(101, &alice_address)?;
    // client.generate_to_address(101, &hetman_address)?;

    // let min_conf = 1;
    // let max_conf = 9999999;
    // let include_unsafe = true;
    // let query_options = json!({});

    // // List UTXOx para Hetman
    // let unspet_hetman: Vec<serde_json::Value> = bitcoind.client.call(
    //     "listunspent",
    //     &[
    //         json!(min_conf),
    //         json!(max_conf),
    //         json!(vec![hetman_address.to_string()]),
    //         json!(include_unsafe),
    //         query_options.clone(),
    //     ],
    // )?;

    // println!("UTXOs disponibles para Hetman: {:?}", unspet_hetman);

    // List UTXOs
    // let unspent: Vec<serde_json::Value> = client.call(
    //     "listunspent",
    //     &[
    //         json!(min_conf),
    //         json!(max_conf),
    //         json!(vec![alice_address.to_string()]),
    //         json!(include_unsafe),
    //         query_options.clone(),
    //     ],
    // )?;
    // println!("UTXOs disponibles: {:?}", unspent);

    // let utxo_amount = Amount::from_btc(first_unspent["amount"].as_f64().unwrap())?;

    // println!("UTXO amount: {:?}", utxo_amount);

    // Check if the UTXO amount is 50 BTC
    // assert_eq!(
    //     utxo_amount.to_sat(),
    //     5_000_000_000,
    //     "UTXO amount is not 50 BTC"
    // );

    // let txin = TxIn {
    //     previous_output: OutPoint::new(
    //         first_unspent["txid"].as_str().unwrap().parse()?,
    //         first_unspent["vout"].as_u64().unwrap() as u32,
    //     ),
    //     script_sig: ScriptBuf::new(), // Initially empty, will be filled later with the signature
    //     sequence: Sequence::MAX,
    //     witness: Witness::default(),
    // };

    // let txout = TxOut {
    //     value: SPEND_AMOUNT,
    //     script_pubkey: bob_script_pubkey.clone(),
    // };

    // println!("txout: {:?}", txout);

    // let change_amount = utxo_amount - SPEND_AMOUNT - Amount::from_sat(1000); // 1000 satoshis for fee

    // println!("change_amount: {:?}", change_amount);

    // let change_txout = TxOut {
    //     value: change_amount,
    //     script_pubkey: alice_script_pubkey.clone(),
    // };

    // let tx = Transaction {
    //     version: transaction::Version::ONE,
    //     lock_time: absolute::LockTime::Blocks(Height::from_consensus(1).unwrap()),
    //     input: vec![txin],
    //     output: vec![txout, change_txout],
    // };
    // println!("tx: {:?}", tx);

    // let unsigned_tx_hex = bitcoin::consensus::encode::serialize_hex(&tx);
    // println!("unsigned_tx_hex: {:?}", unsigned_tx_hex);

    // // client.call("loadwallet", &["alice".into()])?;

    // let signed_tx_result: Value =
    //     client.call("signrawtransactionwithwallet", &[unsigned_tx_hex.into()])?;

    // let signed_tx_hex = signed_tx_result["hex"]
    //     .as_str()
    //     .expect("No signed transaction hex found");

    // println!("signed_tx_hex: {:?}", signed_tx_hex);

    // let signed_tx: Transaction =
    //     bitcoin::consensus::encode::deserialize(&hex::decode(signed_tx_hex)?)?;

    // println!("signed_tx: {:?}", signed_tx);

    // Get the sighash to sign.
    // let sighash_type = EcdsaSighashType::All;
    // let sighasher = SighashCache::new(tx.clone());
    // let sighash = sighasher
    //     .legacy_signature_hash(0, &alice_script_pubkey, sighash_type.to_u32())
    //     .expect("failed to create sighash");

    // println!("sighash: {:?}", sighash);

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

// HELP
// let recipient_sk = SecretKey::new(&mut rand::thread_rng());
// let recipient_pk = bitcoin::PublicKey::new(recipient_sk.public_key(&secp));
// let recipient_address = Address::p2pkh(&recipient_pk, Network::Regtest);
// println!("Generated address (recipient): {}", recipient_address);

#[tokio::test]
#[ignore]
async fn submit_transaction() -> Result<()> {
    let bitcoind = bitcoind::BitcoinD::from_downloaded().unwrap();

    let secp = Secp256k1::new();

    let (sk, wpkh) = senders_keys(&secp);

    // Get an address to send to.
    let address = receivers_address();

    // Get an unspent output that is locked to the key above that we control.
    // In a real application these would come from the chain.
    let (dummy_out_point, dummy_utxo) = dummy_unspent_transaction_output(wpkh);

    // The input for the transaction we are constructing.
    let input = TxIn {
        previous_output: dummy_out_point, // The dummy output we are spending.
        script_sig: ScriptBuf::default(), // For a p2wpkh script_sig is empty.
        sequence: Sequence::ENABLE_RBF_NO_LOCKTIME,
        witness: Witness::default(), // Filled in after signing.
    };

    // The spend output is locked to a key controlled by the receiver.
    let spend = TxOut {
        value: SPEND_AMOUNT,
        script_pubkey: address.script_pubkey(),
    };

    // The change output is locked to a key controlled by us.
    let change = TxOut {
        value: CHANGE_AMOUNT,
        script_pubkey: ScriptBuf::new_p2wpkh(&wpkh), // Change comes back to us.
    };

    // The transaction we want to sign and broadcast.
    let mut unsigned_tx = Transaction {
        version: transaction::Version::TWO,  // Post BIP-68.
        lock_time: absolute::LockTime::ZERO, // Ignore the locktime.
        input: vec![input],                  // Input goes into index 0.
        output: vec![spend, change],         // Outputs, order does not matter.
    };
    let input_index = 0;

    // Get the sighash to sign.
    let sighash_type = EcdsaSighashType::All;
    let mut sighasher = SighashCache::new(&mut unsigned_tx);
    let sighash = sighasher
        .p2wpkh_signature_hash(
            input_index,
            &dummy_utxo.script_pubkey,
            DUMMY_UTXO_AMOUNT,
            sighash_type,
        )
        .expect("failed to create sighash");

    // Sign the sighash using the secp256k1 library (exported by rust-bitcoin).
    let msg = Message::from(sighash);
    let signature = secp.sign_ecdsa(&msg, &sk);

    // Update the witness stack.
    let signature = bitcoin::ecdsa::Signature {
        signature,
        sighash_type,
    };
    let pk = sk.public_key(&secp);
    *sighasher.witness_mut(input_index).unwrap() = Witness::p2wpkh(&signature, &pk);

    // Get the signed transaction.
    let tx = sighasher.into_transaction();

    // BOOM! Transaction signed and ready to broadcast.
    println!("{:#?}", tx);

    let raw_tx_result = bitcoind.client.send_raw_transaction(tx).unwrap();

    println!("{:#?}", raw_tx_result);

    Ok(())
}

/// An example of keys controlled by the transaction sender.
fn senders_keys<C: Signing>(secp: &Secp256k1<C>) -> (SecretKey, WPubkeyHash) {
    let sk = SecretKey::new(&mut rand::thread_rng());
    let pk = bitcoin::PublicKey::new(sk.public_key(secp));
    let wpkh = pk.wpubkey_hash().expect("key is compressed");

    (sk, wpkh)
}

/// A dummy address for the receiver.
///
/// We lock the spend output to the key associated with this address.
///
/// (FWIW this is a random mainnet address from block 80219.)
fn receivers_address() -> Address {
    "bc1q7cyrfmck2ffu2ud3rn5l5a8yv6f0chkp0zpemf"
        .parse::<Address<_>>()
        .expect("a valid address")
        .require_network(Network::Bitcoin)
        .expect("valid address for mainnet")
}

/// Creates a p2wpkh output locked to the key associated with `wpkh`.
///
/// An utxo is described by the `OutPoint` (txid and index within the transaction that it was
/// created). Using the out point one can get the transaction by `txid` and using the `vout` get the
/// transaction value and script pubkey (`TxOut`) of the utxo.
///
/// This output is locked to keys that we control, in a real application this would be a valid
/// output taken from a transaction that appears in the chain.
fn dummy_unspent_transaction_output(wpkh: WPubkeyHash) -> (OutPoint, TxOut) {
    let script_pubkey = ScriptBuf::new_p2wpkh(&wpkh);

    let out_point = OutPoint {
        txid: Txid::all_zeros(), // Obviously invalid.
        vout: 0,
    };

    let utxo = TxOut {
        value: DUMMY_UTXO_AMOUNT,
        script_pubkey,
    };

    (out_point, utxo)
}

// ASD ASD ASD
// #[tokio::test]
// async fn submit_transaction() -> Result<()> {
//     let bitcoind = bitcoind::BitcoinD::from_downloaded().unwrap();

//     // Generate some blocks to get coins
//     bitcoind.client.generate_to_address(101, &bitcoind.client.get_new_address(None, None)?)?;

//     // Get a real UTXO
//     let unspent = bitcoind.client.list_unspent(None, None, None, None, None)?
//         .into_iter()
//         .next()
//         .expect("There should be at least one unspent output");

//     let secp = Secp256k1::new();
//     let (sk, _) = senders_keys(&secp);

//     // Use the real UTXO details
//     let dummy_out_point = OutPoint::new(unspent.txid, unspent.vout);
//     let dummy_utxo = TxOut {
//         value: Amount::from_btc(unspent.amount)?,
//         script_pub_key: unspent.script_pub_key,
//     };

//     // Rest of your transaction creation code...
//     // Make sure to use the correct amounts based on the real UTXO
// }
