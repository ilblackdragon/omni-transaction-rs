pub trait TxBuilder<T> {
    fn build(&self) -> T;
}

pub struct TransactionBuilder;

impl TransactionBuilder {
    #[allow(clippy::new_ret_no_self)]
    pub fn new<T>() -> T
    where
        T: Default,
    {
        T::default()
    }
}

#[cfg(test)]
mod tests {

    use super::{TransactionBuilder as OmniTransactionBuilder, TxBuilder};
    use crate::{
        evm::utils::parse_eth_address,
        types::{EVM, NEAR},
    };
    use alloy::{
        consensus::SignableTransaction,
        network::TransactionBuilder,
        primitives::{address, hex, Address, U256},
        rpc::types::TransactionRequest,
    };

    #[test]
    fn test_near_transaction_builder_typed() {
        let near_transaction = OmniTransactionBuilder::new::<NEAR>()
            .nonce(0)
            .sender_id("alice.near".to_string())
            .signer_public_key([0u8; 64])
            .receiver_id("alice.near".to_string())
            .build();

        let tx_encoded = near_transaction.build_for_signing();

        assert_eq!(hex::encode(tx_encoded), "0a000000616c6963652e6e656172010000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000a000000616c6963652e6e656172000000000000000000000000000000000000000000000000000000000000000000000000");
    }

    #[test]
    fn test_evm_transaction_builder_typed() {
        const MAX_FEE_PER_GAS: u128 = 20_000_000_000;
        const MAX_PRIORITY_FEE_PER_GAS: u128 = 1_000_000_000;
        const GAS_LIMIT: u128 = 21_000;

        let nonce: u64 = 0;
        let to: Address = address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045");
        let value = 10000000000000000u128; // 0.01 ETH
        let data: Vec<u8> = vec![];
        let chain_id = 1;
        let to_address_str = "d8dA6BF26964aF9D7eEd9e03E53415D37aA96045";
        let to_address = parse_eth_address(to_address_str);

        let tx = OmniTransactionBuilder::new::<EVM>()
            .chain_id(chain_id)
            .nonce(nonce)
            .max_priority_fee_per_gas(MAX_PRIORITY_FEE_PER_GAS)
            .max_fee_per_gas(MAX_FEE_PER_GAS)
            .gas_limit(GAS_LIMIT)
            .to(to_address)
            .value(value)
            .input(data.clone())
            .access_list(vec![])
            .build();

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

        let mut rlp_alloy_bytes = vec![];
        rlp_encoded.encode_for_signing(&mut rlp_alloy_bytes);

        assert!(rlp_alloy_bytes == rlp_bytes);
    }
}
