//! Transaction builder, encoders, types and utilities for Aptos.
//!
//! An [`AptosTransaction`] is aptos-core's `RawTransaction`:
//! [`AptosTransaction::build_for_signing`] returns the Ed25519 signing
//! message `sha3_256("APTOS::RawTransaction") || bcs(raw_txn)` (the full
//! preimage — sign it directly, do not hash it again), and
//! [`AptosTransaction::build_with_signature`] returns the BCS
//! `SignedTransaction` bytes ready to broadcast via
//! `POST {fullnode}/v1/transactions` with
//! `Content-Type: application/x.aptos.signed_transaction+bcs`.
//!
//! ###### Example:
//!
//! ```rust
//! use omni_transaction::aptos::types::{
//!     AccountAddress, EntryFunction, Identifier, ModuleId, TransactionPayload,
//! };
//! use omni_transaction::{TransactionBuilder, TxBuilder, APTOS};
//!
//! let sender = AccountAddress::from_hex("0xa550c18").unwrap();
//! let receiver = AccountAddress::from_hex("0xdd").unwrap();
//!
//! // Each entry-function argument is the BCS encoding of its value:
//! // 32 raw bytes for an address, 8 little-endian bytes for a u64 amount.
//! let payload = TransactionPayload::EntryFunction(EntryFunction::new(
//!     ModuleId::new(AccountAddress::ONE, Identifier::new("aptos_account").unwrap()),
//!     Identifier::new("transfer").unwrap(),
//!     vec![],
//!     vec![receiver.as_bytes().to_vec(), 1_000u64.to_le_bytes().to_vec()],
//! ));
//!
//! let aptos_tx = TransactionBuilder::new::<APTOS>()
//!     .sender(sender)
//!     .sequence_number(0)
//!     .payload(payload)
//!     .max_gas_amount(2000)
//!     .gas_unit_price(100)
//!     .expiration_timestamp_secs(1_735_689_600)
//!     .chain_id(1) // mainnet
//!     .build();
//!
//! // The payload to sign with Ed25519 (e.g. via NEAR MPC chain signatures).
//! let signing_message = aptos_tx.build_for_signing();
//! assert_eq!(signing_message.len(), 32 + aptos_tx.to_bcs_bytes().len());
//! ```
mod aptos_transaction;
mod aptos_transaction_builder;
pub mod constants;
pub mod types;

/// Aptos transaction
pub use aptos_transaction::AptosTransaction;
/// Aptos transaction builder
pub use aptos_transaction_builder::AptosTransactionBuilder;
