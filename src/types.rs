use crate::evm::evm_transaction_builder::EVMTransactionBuilder;
use crate::near::near_transaction_builder::NearTransactionBuilder;

pub type Address = [u8; 20];

pub type AccessList = Vec<(Address, Vec<[u8; 32]>)>;

pub struct Signature {
    pub v: u64,
    pub r: Vec<u8>,
    pub s: Vec<u8>,
}

pub type NEAR = NearTransactionBuilder;

pub type EVM = EVMTransactionBuilder;
