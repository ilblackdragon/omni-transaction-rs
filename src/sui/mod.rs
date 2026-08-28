//! Transaction builder, encoders, types and utilities for Sui.
//!
//! Transactions are hand-encoded as BCS (Binary Canonical Serialization) and
//! signed over the 32-byte Blake2b-256 digest of the intent message; see
//! [`SuiTransaction::build_for_signing`] — unlike the EVM/Bitcoin modules it
//! returns the final digest, not a preimage.
//!
//! ###### Example:
//!
//! ```rust
//! use omni_transaction::sui::types::{Argument, CallArg, Command, ObjectDigest, ObjectRef};
//! use omni_transaction::sui::utils::parse_sui_address;
//! use omni_transaction::{TransactionBuilder, TxBuilder, SUI};
//!
//! // Send 1_000_000 MIST from the gas coin to a recipient.
//! let sui_tx = TransactionBuilder::new::<SUI>()
//!     .sender(parse_sui_address("0x2"))
//!     .programmable(
//!         vec![
//!             CallArg::pure_u64(1_000_000),
//!             CallArg::pure_address(parse_sui_address("0x3")),
//!         ],
//!         vec![
//!             Command::SplitCoins {
//!                 coin: Argument::GasCoin,
//!                 amounts: vec![Argument::Input(0)],
//!             },
//!             Command::TransferObjects {
//!                 objects: vec![Argument::Result(0)],
//!                 address: Argument::Input(1),
//!             },
//!         ],
//!     )
//!     .gas_payment(vec![ObjectRef::new(
//!         parse_sui_address("0x1"),
//!         2,
//!         ObjectDigest::new([0x63u8; 32]),
//!     )])
//!     .gas_price(1000)
//!     .gas_budget(5_000_000)
//!     .build();
//!
//! // 32-byte blake2b-256 digest to pass to an ed25519 signer (e.g. NEAR MPC).
//! let digest = sui_tx.build_for_signing();
//! # assert_eq!(digest.len(), 32);
//! ```
mod sui_transaction;
mod sui_transaction_builder;
pub mod types;
pub mod utils;

/// Sui transaction
pub use sui_transaction::SuiTransaction;
/// Sui transaction builder
pub use sui_transaction_builder::SuiTransactionBuilder;
