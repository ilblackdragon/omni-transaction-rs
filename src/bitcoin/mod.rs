//! Transaction builder, encoders, types and utilities for Bitcoin.
mod bitcoin_transaction;
mod bitcoin_transaction_builder;
mod constants;
pub(crate) mod encoding;
pub mod types;
pub mod utils;

/// Bitcoin transaction
pub use bitcoin_transaction::BitcoinTransaction;
/// Bitcoin transaction builder
pub use bitcoin_transaction_builder::BitcoinTransactionBuilder;
