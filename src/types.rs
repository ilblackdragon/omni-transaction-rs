#[cfg(feature = "bitcoin")]
use crate::bitcoin::bitcoin_transaction_builder::BitcoinTransactionBuilder;

#[cfg(feature = "evm")]
use crate::evm::evm_transaction_builder::EVMTransactionBuilder;

#[cfg(feature = "near")]
use crate::near::near_transaction_builder::NearTransactionBuilder;

#[cfg(feature = "near")]
pub type NEAR = NearTransactionBuilder;

#[cfg(feature = "evm")]
pub type EVM = EVMTransactionBuilder;

#[cfg(feature = "bitcoin")]
pub type BITCOIN = BitcoinTransactionBuilder;
