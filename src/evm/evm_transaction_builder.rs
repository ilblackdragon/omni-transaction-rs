use crate::transaction_builder::TxBuilder;

use super::{
    evm_transaction::EVMTransaction,
    types::{AccessList, Address},
};

pub struct EVMTransactionBuilder {
    chain_id: Option<u64>,
    nonce: Option<u64>,
    to: Option<Address>,
    value: Option<u128>,
    input: Option<Vec<u8>>,
    gas_limit: Option<u128>,
    max_fee_per_gas: Option<u128>,
    max_priority_fee_per_gas: Option<u128>,
    access_list: Option<AccessList>,
}

impl Default for EVMTransactionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl TxBuilder<EVMTransaction> for EVMTransactionBuilder {
    fn build(&self) -> EVMTransaction {
        EVMTransaction {
            chain_id: self.chain_id.expect("chain_id is mandatory"),
            nonce: self.nonce.expect("nonce is mandatory"),
            to: self.to,
            value: self.value.unwrap_or_default(),
            input: self.input.clone().unwrap_or_default(),
            gas_limit: self.gas_limit.expect("gas_limit is mandatory"),
            max_fee_per_gas: self.max_fee_per_gas.expect("max_fee_per_gas is mandatory"),
            max_priority_fee_per_gas: self.max_priority_fee_per_gas.unwrap_or_default(),
            access_list: self.access_list.clone().unwrap_or_default(),
        }
    }
}

impl EVMTransactionBuilder {
    pub const fn new() -> Self {
        Self {
            chain_id: None,
            nonce: None,
            to: None,
            value: None,
            input: None,
            gas_limit: None,
            max_fee_per_gas: None,
            max_priority_fee_per_gas: None,
            access_list: None,
        }
    }

    /// Chain ID of the transaction.
    pub const fn chain_id(mut self, chain_id: u64) -> Self {
        self.chain_id = Some(chain_id);
        self
    }

    /// Nonce of the transaction.
    pub const fn nonce(mut self, nonce: u64) -> Self {
        self.nonce = Some(nonce);
        self
    }

    /// Address of the recipient.
    pub const fn to(mut self, to: Address) -> Self {
        self.to = Some(to);
        self
    }

    /// Value attached to the transaction.
    pub const fn value(mut self, value: u128) -> Self {
        self.value = Some(value);
        self
    }

    /// Input data of the transaction.
    pub fn input(mut self, input: Vec<u8>) -> Self {
        self.input = Some(input);
        self
    }

    /// Gas limit of the transaction.
    pub const fn gas_limit(mut self, gas_limit: u128) -> Self {
        self.gas_limit = Some(gas_limit);
        self
    }

    /// Maximum fee per gas of the transaction.
    pub const fn max_fee_per_gas(mut self, max_fee_per_gas: u128) -> Self {
        self.max_fee_per_gas = Some(max_fee_per_gas);
        self
    }

    /// Maximum priority fee per gas of the transaction.
    pub const fn max_priority_fee_per_gas(mut self, max_priority_fee_per_gas: u128) -> Self {
        self.max_priority_fee_per_gas = Some(max_priority_fee_per_gas);
        self
    }

    /// Access list of the transaction.
    pub fn access_list(mut self, access_list: AccessList) -> Self {
        self.access_list = Some(access_list);
        self
    }
}

#[cfg(test)]
mod tests {
    use alloy::{
        consensus::SignableTransaction,
        network::TransactionBuilder,
        primitives::{address, hex, Address, Bytes, U256},
        rpc::types::{AccessList, TransactionRequest},
    };

    use crate::{
        evm::{evm_transaction_builder::EVMTransactionBuilder, utils::parse_eth_address},
        transaction_builder::TxBuilder,
    };

    const MAX_FEE_PER_GAS: u128 = 20_000_000_000;
    const MAX_PRIORITY_FEE_PER_GAS: u128 = 1_000_000_000;
    const GAS_LIMIT: u128 = 21_000;

    #[test]
    fn test_evm_transaction_builder_against_alloy() {
        let nonce: u64 = 0;
        let to: Address = address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045");
        let value = 10000000000000000u128; // 0.01 ETH
        let data: Vec<u8> = vec![];
        let chain_id = 1;
        let to_address_str = "d8dA6BF26964aF9D7eEd9e03E53415D37aA96045";
        let to_address = parse_eth_address(to_address_str);

        // Generate using EVMTransactionBuilder
        let tx_1 = EVMTransactionBuilder::new()
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

        let rlp_bytes = tx_1.build_for_signing();

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

    #[test]
    fn test_evm_transaction_builder_with_data_against_alloy() {
        let input: Bytes = hex!("a22cb4650000000000000000000000005eee75727d804a2b13038928d36f8b188945a57a0000000000000000000000000000000000000000000000000000000000000000").into();
        let nonce: u64 = 0;
        let to: Address = address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045");
        let value = 10000000000000000u128; // 0.01 ETH
        let chain_id = 1;
        let to_address_str = "d8dA6BF26964aF9D7eEd9e03E53415D37aA96045";
        let to_address = parse_eth_address(to_address_str);

        // Generate using EVMTransactionBuilder
        let evm_transaction = EVMTransactionBuilder::new()
            .chain_id(chain_id)
            .nonce(nonce)
            .max_priority_fee_per_gas(MAX_PRIORITY_FEE_PER_GAS)
            .max_fee_per_gas(MAX_FEE_PER_GAS)
            .gas_limit(GAS_LIMIT)
            .to(to_address)
            .value(value)
            .input(input.to_vec())
            .access_list(vec![])
            .build();

        let rlp_bytes = evm_transaction.build_for_signing();

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
        let mut rlp_encoded_encoded_for_signing = vec![];
        rlp_encoded.encode_for_signing(&mut rlp_encoded_encoded_for_signing);

        assert!(rlp_encoded_encoded_for_signing == rlp_bytes);
    }
}
