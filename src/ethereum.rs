use rlp::{Encodable, RlpStream};
use hex;

type Address = [u8; 20];
type AccessList = Vec<(Address, Vec<[u8; 32]>)>;

const EIP_1559_TYPE: u8 = 0x02;

pub fn parse_eth_address(address: &str) -> Address {
    let address = hex::decode(address).expect("address should be hex");
    assert_eq!(address.len(), 20, "address should be 20 bytes long");
    let mut result = [0u8; 20];
    result.copy_from_slice(&address);
    result
}

pub fn ethereum_transaction(chain: u64, nonce: u128, max_priority_fee_per_gas: u128, max_fee_per_gas: u128, gas: u128, to: Option<Address>, value: u128, data: Vec<u8>, access_list: AccessList) -> Vec<u8> {
    let mut rlp_stream = RlpStream::new();

    let to: Vec<u8> = match to {
        Some(ref to) => to.to_vec(),
        None => vec![],
    };

    rlp_stream.begin_unbounded_list();
    rlp_stream.append(&chain);
    rlp_stream.append(&nonce);
    rlp_stream.append(&max_priority_fee_per_gas);
    rlp_stream.append(&max_fee_per_gas);
    rlp_stream.append(&gas);
    rlp_stream.append(&to);
    rlp_stream.append(&value);
    rlp_stream.append(&data);

    // Write access list.
    {
        rlp_stream.begin_unbounded_list();
        for access in access_list {
            rlp_stream.begin_unbounded_list();
            rlp_stream.append(&access.0.to_vec());
            // Append list of storage keys.
            {
                rlp_stream.begin_unbounded_list();
                for storage_key in access.1 {
                    rlp_stream.append(&storage_key.to_vec());
                }
                rlp_stream.finalize_unbounded_list();
            }
            rlp_stream.finalize_unbounded_list();
        }
        rlp_stream.finalize_unbounded_list();
    }

    rlp_stream.finalize_unbounded_list();
    let mut rlp_bytes = rlp_stream.out().to_vec();
    // Insert the type of transaction as the first byte.
    rlp_bytes.insert(0usize, EIP_1559_TYPE);
    rlp_bytes
}
