//! Transaction builder, encoders, types and utilities for Zcash
//! (transparent-only v5 transactions, [ZIP-225] format with [ZIP-244]
//! txid/signature digests).
//!
//! [ZIP-225]: https://zips.z.cash/zip-0225
//! [ZIP-244]: https://zips.z.cash/zip-0244
mod constants;
mod sighash;
pub mod types;
pub mod utils;
mod zcash_transaction;
mod zcash_transaction_builder;

/// Zcash transaction
pub use zcash_transaction::ZcashTransaction;
/// Zcash transaction builder
pub use zcash_transaction_builder::ZcashTransactionBuilder;
