use bitcoin::{bip32::Xpriv, Address, Network, ScriptBuf};
use bitcoind::AddressType;
use serde_json::Value;

use std::{result::Result::Ok, str::FromStr as _};

pub fn get_address_info_for(
    client: &bitcoind::Client,
    name: &str,
) -> Result<(Address, ScriptBuf), Box<dyn std::error::Error>> {
    let address = client
        .get_new_address_with_type(AddressType::Legacy)
        .unwrap()
        .address()
        .unwrap();

    let address = address.require_network(Network::Regtest).unwrap();
    println!("{} address: {:?}", name, address);

    // Get address info for Bob
    let address_info: Value = client.call("getaddressinfo", &[address.to_string().into()])?;
    println!("{} Address info: {:?}", name, address_info);

    // Extract the scriptPubKey from the result
    let script_pubkey_hex = address_info["scriptPubKey"]
        .as_str()
        .expect("scriptPubKey should be a string");

    let script_pubkey =
        ScriptBuf::from_hex(script_pubkey_hex).expect("Failed to parse scriptPubKey");

    println!("{} ScriptPubKey: {:?}", name, script_pubkey);

    Ok((address, script_pubkey))
}

pub fn get_master_key_of_regtest_node(
    client: &bitcoind::Client,
) -> Result<Xpriv, Box<dyn std::error::Error>> {
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

    let master_key = Xpriv::from_str(&master_key_str).unwrap();
    println!("Master key: {}", master_key);

    Ok(master_key)
}
