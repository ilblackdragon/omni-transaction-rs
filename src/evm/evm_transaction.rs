use near_sdk::serde::{Deserialize, Serialize};
use rlp::RlpStream;

use crate::constants::EIP_1559_TYPE;

use super::types::{AccessList, Address, Signature};

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct EVMTransaction {
    pub chain_id: u64,
    pub nonce: u64,
    pub to: Option<Address>,
    pub value: u128,
    pub input: Vec<u8>,
    pub gas_limit: u128,
    pub max_fee_per_gas: u128,
    pub max_priority_fee_per_gas: u128,
    pub access_list: AccessList,
}

impl EVMTransaction {
    pub fn build_for_signing(&self) -> Vec<u8> {
        let mut rlp_stream = RlpStream::new();

        rlp_stream.append(&EIP_1559_TYPE);

        rlp_stream.begin_unbounded_list();

        self.encode_fields(&mut rlp_stream);

        rlp_stream.finalize_unbounded_list();

        rlp_stream.out().to_vec()
    }

    pub fn build_with_signature(&self, signature: &Signature) -> Vec<u8> {
        let mut rlp_stream = RlpStream::new();

        rlp_stream.append(&EIP_1559_TYPE);

        rlp_stream.begin_unbounded_list();

        self.encode_fields(&mut rlp_stream);

        rlp_stream.append(&signature.v);
        rlp_stream.append(&signature.r);
        rlp_stream.append(&signature.s);

        rlp_stream.finalize_unbounded_list();

        rlp_stream.out().to_vec()
    }

    fn encode_fields(&self, rlp_stream: &mut RlpStream) {
        let to: Vec<u8> = self.to.map_or(vec![], |to| to.to_vec());
        let access_list = self.access_list.clone();

        rlp_stream.append(&self.chain_id);
        rlp_stream.append(&self.nonce);
        rlp_stream.append(&self.max_priority_fee_per_gas);
        rlp_stream.append(&self.max_fee_per_gas);
        rlp_stream.append(&self.gas_limit);
        rlp_stream.append(&to);
        rlp_stream.append(&self.value);
        rlp_stream.append(&self.input);

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
    }
}

#[cfg(test)]
mod tests {
    use alloy::{
        consensus::{SignableTransaction, TxEip1559},
        network::TransactionBuilder,
        primitives::{address, hex, Address, Bytes, U256},
        rpc::types::{AccessList, TransactionRequest},
    };
    use alloy_primitives::{b256, Signature};

    use crate::evm::types::Signature as OmniSignature;
    use crate::evm::{evm_transaction::EVMTransaction, utils::parse_eth_address};
    const MAX_FEE_PER_GAS: u128 = 20_000_000_000;
    const MAX_PRIORITY_FEE_PER_GAS: u128 = 1_000_000_000;
    const GAS_LIMIT: u128 = 21_000;

    #[test]
    fn test_build_for_signing_for_evm_against_alloy() {
        let nonce: u64 = 0;
        let to: Address = address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045");
        let value = 10000000000000000u128; // 0.01 ETH
        let data: Vec<u8> = vec![];
        let chain_id = 1;
        let to_address_str = "d8dA6BF26964aF9D7eEd9e03E53415D37aA96045";
        let to_address = Some(parse_eth_address(to_address_str));

        // Generate using EVMTransaction
        let tx = EVMTransaction {
            chain_id,
            nonce,
            to: to_address,
            value,
            input: data.clone(),
            gas_limit: GAS_LIMIT,
            max_fee_per_gas: MAX_FEE_PER_GAS,
            max_priority_fee_per_gas: MAX_PRIORITY_FEE_PER_GAS,
            access_list: vec![],
        };

        let rlp_bytes = tx.build_for_signing();

        // Now let's compare with the Alloy RLP encoding
        let alloy_tx = TransactionRequest::default()
            .with_chain_id(chain_id)
            .with_nonce(nonce)
            .with_to(to)
            .with_value(U256::from(value))
            .with_max_priority_fee_per_gas(MAX_PRIORITY_FEE_PER_GAS)
            .with_max_fee_per_gas(MAX_FEE_PER_GAS)
            .with_gas_limit(GAS_LIMIT)
            .with_input(data);

        let alloy_rlp_bytes: alloy::consensus::TypedTransaction = alloy_tx
            .build_unsigned()
            .expect("Failed to build unsigned transaction");

        let rlp_encoded = alloy_rlp_bytes.eip1559().unwrap();

        // Prepare the buffer and encode
        let mut buf = vec![];
        rlp_encoded.encode_for_signing(&mut buf);

        assert!(buf == rlp_bytes);
    }

    #[test]
    fn test_build_for_signing_with_data_for_evm_against_alloy() {
        let input: Bytes = hex!("a22cb4650000000000000000000000005eee75727d804a2b13038928d36f8b188945a57a0000000000000000000000000000000000000000000000000000000000000000").into();
        let nonce: u64 = 0;
        let to: Address = address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045");
        let value = 10000000000000000u128; // 0.01 ETH
        let chain_id = 1;
        let to_address_str = "d8dA6BF26964aF9D7eEd9e03E53415D37aA96045";
        let to_address = Some(parse_eth_address(to_address_str));

        // Generate using EVMTransaction
        let tx = EVMTransaction {
            chain_id,
            nonce,
            to: to_address,
            value,
            input: input.to_vec(),
            gas_limit: GAS_LIMIT,
            max_fee_per_gas: MAX_FEE_PER_GAS,
            max_priority_fee_per_gas: MAX_PRIORITY_FEE_PER_GAS,
            access_list: vec![],
        };

        let rlp_bytes = tx.build_for_signing();

        // Now let's compare with the Alloy RLP encoding
        let alloy_tx = TransactionRequest::default()
            .with_chain_id(chain_id)
            .with_nonce(nonce)
            .with_to(to)
            .with_value(U256::from(value))
            .with_max_priority_fee_per_gas(MAX_PRIORITY_FEE_PER_GAS)
            .with_max_fee_per_gas(MAX_FEE_PER_GAS)
            .with_gas_limit(GAS_LIMIT)
            .access_list(AccessList::default())
            .with_input(input);

        let alloy_rlp_bytes: alloy::consensus::TypedTransaction = alloy_tx
            .build_unsigned()
            .expect("Failed to build unsigned transaction");

        let rlp_encoded = alloy_rlp_bytes.eip1559().unwrap();

        // Prepare the buffer and encode
        let mut buf = vec![];
        rlp_encoded.encode_for_signing(&mut buf);

        assert!(buf == rlp_bytes);
    }

    #[test]
    fn test_build_with_signature_for_evm_against_alloy() {
        let chain_id = 1;
        let nonce = 0x42;
        let gas_limit = 44386;

        let to_str = "6069a6c32cf691f5982febae4faf8a6f3ab2f0f6";
        let to = address!("6069a6c32cf691f5982febae4faf8a6f3ab2f0f6").into();
        let to_address = Some(parse_eth_address(to_str));
        let value_as_128 = 0_u128;
        let value = U256::from(value_as_128);

        let max_fee_per_gas = 0x4a817c800;
        let max_priority_fee_per_gas = 0x3b9aca00;
        let input: Bytes = hex!("a22cb4650000000000000000000000005eee75727d804a2b13038928d36f8b188945a57a0000000000000000000000000000000000000000000000000000000000000000").into();

        let tx: TxEip1559 = TxEip1559 {
            chain_id,
            nonce,
            gas_limit,
            to,
            value,
            input: input.clone(),
            max_fee_per_gas,
            max_priority_fee_per_gas,
            access_list: AccessList::default(),
        };

        let mut tx_encoded = vec![];
        tx.encode_for_signing(&mut tx_encoded);

        // Generate using EVMTransaction
        let tx_omni = EVMTransaction {
            chain_id,
            nonce,
            to: to_address,
            value: value_as_128,
            input: input.to_vec(),
            gas_limit,
            max_fee_per_gas,
            max_priority_fee_per_gas,
            access_list: vec![],
        };

        let rlp_bytes_for_omni_tx = tx_omni.build_for_signing();

        assert_eq!(tx_encoded.len(), rlp_bytes_for_omni_tx.len());

        let sig = Signature::from_scalars_and_parity(
            b256!("840cfc572845f5786e702984c2a582528cad4b49b2a10b9db1be7fca90058565"),
            b256!("25e7109ceb98168d95b09b18bbf6b685130e0562f233877d492b94eee0c5b6d1"),
            false,
        )
        .unwrap();

        let mut tx_encoded_with_signature: Vec<u8> = vec![];
        tx.encode_with_signature(&sig, &mut tx_encoded_with_signature, false);

        let signature: OmniSignature = OmniSignature {
            v: sig.v().to_u64(),
            r: sig.r().to_be_bytes::<32>().to_vec(),
            s: sig.s().to_be_bytes::<32>().to_vec(),
        };

        let omni_encoded_with_signature = tx_omni.build_with_signature(&signature);

        assert_eq!(
            tx_encoded_with_signature.len(),
            omni_encoded_with_signature.len()
        );
        assert_eq!(tx_encoded_with_signature, omni_encoded_with_signature);
    }
}
