use crate::evm::evm_transaction_builder::EVMTransactionBuilder;
use crate::near::near_transaction_builder::NearTransactionBuilder;

pub type NEAR = NearTransactionBuilder;

pub type EVM = EVMTransactionBuilder;
