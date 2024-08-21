use hex;

use super::types::Address;

pub fn parse_eth_address(address: &str) -> Address {
    let address = hex::decode(address).expect("address should be hex");
    assert_eq!(address.len(), 20, "address should be 20 bytes long");
    let mut result = [0u8; 20];
    result.copy_from_slice(&address);
    result
}
