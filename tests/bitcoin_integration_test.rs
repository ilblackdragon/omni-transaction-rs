use bitcoin::absolute::Height;
// use bitcoin::address::script_pubkey::ScriptBufExt as _;
// use bitcoin::blockdata::opcodes::all::{OP_CHECKSIG, OP_DUP, OP_EQUALVERIFY, OP_HASH160};
use bitcoin::blockdata::script::Builder;
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
// use bitcoind::client::client_sync::Auth;
// use bitcoincore_rpc::{Auth, Client as CoreClient, RpcApi};
// use bitcoind::Client;
// use serde_json::json;

use eyre::Result;
use std::result::Result::Ok;

// const DUMMY_UTXO_AMOUNT: Amount = Amount::from_sat(20_000_000);
// const SPEND_AMOUNT: Amount = Amount::from_sat(5_000_000);
// const CHANGE_AMOUNT: Amount = Amount::from_sat(14_999_000); // 1000 sat fee.

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

#[tokio::test]
async fn test_send_p2pkh() -> Result<()> {
    let bitcoind = bitcoind::BitcoinD::from_downloaded().unwrap();
    let client = &bitcoind.client;

    // Generate keys and address BEFORE mining
    let secp = Secp256k1::new();
    let sk = SecretKey::new(&mut rand::thread_rng());
    let pk: bitcoin::PublicKey = bitcoin::PublicKey::new(sk.public_key(&secp));
    let address = Address::p2pkh(&pk, Network::Regtest);
    let script_pubkey = address.script_pubkey();

    println!("address: {:?}", address);
    println!("public key: {:?}", pk);
    println!("script_pubkey: {:?}", script_pubkey);

    // Generate a new address
    let address = client
        .get_new_address_with_type(AddressType::Legacy)
        .unwrap()
        .address()
        .unwrap();

    let network = Network::Regtest;
    let address = address.require_network(network).unwrap();

    println!("New address: {}", address);

    // Generate 101 blocks to the address
    client.generate_to_address(101, &address)?;

    // Verify balance
    let balance = client.get_balance()?;
    println!(
        "Balance of address: {} satoshis",
        balance.balance().unwrap()
    );

    // List UTXOs
    let unspent: Vec<serde_json::Value> = client.call("listunspent", &[])?;
    println!("UTXOs disponibles: {:?}", unspent);

    // Get the first UTXO
    let first_unspent = unspent
        .into_iter()
        .next()
        .expect("There should be at least one unspent output");

    println!("First UTXO: {:?}", first_unspent);

    // Verify UTXO belongs to our address and has the correct amount
    assert_eq!(
        first_unspent["address"].as_str().unwrap(),
        address.to_string(),
        "UTXO doesn't belong to our address"
    );
    let utxo_amount = Amount::from_btc(first_unspent["amount"].as_f64().unwrap())?;
    assert_eq!(
        utxo_amount.to_sat(),
        5_000_000_000,
        "UTXO amount is not 50 BTC"
    );

    let txin = TxIn {
        previous_output: OutPoint::new(
            first_unspent["txid"].as_str().unwrap().parse()?,
            first_unspent["vout"].as_u64().unwrap() as u32,
        ),
        script_sig: ScriptBuf::new(),
        sequence: Sequence::MAX,
        witness: Witness::default(),
    };

    // Create a new address for the recipient
    let recipient_sk = SecretKey::new(&mut rand::thread_rng());
    let recipient_pk = bitcoin::PublicKey::new(recipient_sk.public_key(&secp));
    let recipient_address = Address::p2pkh(&recipient_pk, Network::Regtest);
    println!("Generated address (recipient): {}", recipient_address);

    let txout = TxOut {
        value: SPEND_AMOUNT,
        script_pubkey: script_pubkey.clone(),
    };

    println!("txout: {:?}", txout);

    let change_amount = utxo_amount - SPEND_AMOUNT - Amount::from_sat(1000); // 1000 satoshis for fee

    let change_txout = TxOut {
        value: change_amount,
        script_pubkey: script_pubkey.clone(),
    };

    let mut tx = Transaction {
        version: transaction::Version::ONE,
        lock_time: absolute::LockTime::Blocks(Height::from_consensus(1).unwrap()),
        input: vec![txin],
        output: vec![txout, change_txout],
    };

    // Get the sighash to sign.
    let sighash_type = EcdsaSighashType::All;
    let sighasher = SighashCache::new(tx.clone());
    let sighash = sighasher
        .legacy_signature_hash(0, &script_pubkey, sighash_type.to_u32())
        .expect("failed to create sighash");

    println!("sighash: {:?}", sighash);

    let msg = Message::from(sighash);
    let signature = secp.sign_ecdsa(&msg, &sk);

    println!("signature: {:?}", signature);

    let signature = bitcoin::ecdsa::Signature {
        signature,
        sighash_type,
    };

    println!("signature: {:?}", signature);
    // Create the script_sig
    let script_sig = Builder::new()
        .push_slice(&signature.serialize())
        .push_key(&pk)
        .into_script();

    println!("script_sig: {:?}", script_sig);

    // Asignar script_sig a txin
    tx.input[0].script_sig = script_sig;

    println!("tx: {:?}", tx);

    // Get the signed transaction
    // let tx_signed = sighasher.into_transaction();

    // println!("tx_signed: {:?}", tx_signed);

    let raw_tx_result = client.send_raw_transaction(&tx).unwrap();

    println!("raw_tx_result: {:?}", raw_tx_result);

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
