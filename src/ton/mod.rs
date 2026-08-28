//! Transaction builder, encoders, types and utilities for TON
//! (The Open Network).
//!
//! Transactions are hand-encoded as TON cells (per the TVM whitepaper and
//! `block.tlb`) targeting the v4r2 and v5r1 (W5) wallet contracts, and
//! signed over the 32-byte representation hash of the unsigned wallet body;
//! see [`TonTransaction::build_for_signing`] — unlike the EVM/Bitcoin
//! modules it returns the final digest, not a preimage, so pass it to an
//! ed25519 signer (e.g. NEAR MPC) without hashing again.
//! [`TonTransaction::build_with_signature`] then returns the Bag of Cells
//! bytes of the signed external message, ready to broadcast (base64-encode
//! them for toncenter's `sendBoc`).
//!
//! ###### Example:
//!
//! ```rust
//! use omni_transaction::ton::types::{Coins, InternalMessage, TonAddress};
//! use omni_transaction::{TransactionBuilder, TxBuilder, TON};
//!
//! // The ed25519 public key that controls the wallet (e.g. a NEAR MPC key).
//! let public_key: [u8; 32] =
//!     hex::decode("31debe55d37c722768b137131caa6087080b2e0b60b94bd785d14575cfa498bc")
//!         .unwrap()
//!         .try_into()
//!         .unwrap();
//! let dest: TonAddress = "EQCD39VS5jcptHL8vMjEXrzGaRcCVYto7HUn4bpAOg8xqB2N"
//!     .parse()
//!     .unwrap();
//!
//! // Send 0.05 TON from a v5r1 wallet (the default version).
//! let ton_tx = TransactionBuilder::new::<TON>()
//!     .public_key(public_key)
//!     .seqno(1)
//!     .valid_until(1735689600)
//!     .add_message(InternalMessage::new(dest, Coins::from_nano(50_000_000)))
//!     .build();
//!
//! // 32-byte hash of the unsigned wallet body — sign with ed25519.
//! let payload = ton_tx.build_for_signing();
//! assert_eq!(
//!     hex::encode(&payload),
//!     "0b45a74ddb8c4bd8f9a504b255869daad1bba81a1bd03351ea5ca1a3c47a2416"
//! );
//!
//! // With the 64-byte signature, produce the broadcastable BoC.
//! let signature = [0u8; 64]; // <- replace with the real signature
//! let boc = ton_tx.build_with_signature(signature);
//! assert_eq!(&boc[..4], &[0xB5, 0xEE, 0x9C, 0x72]);
//! ```
mod constants;
mod ton_transaction;
mod ton_transaction_builder;
pub mod types;
pub mod utils;

/// TON transaction
pub use ton_transaction::TonTransaction;
/// Maximum internal messages per v4r2 transaction
pub use ton_transaction::V4R2_MAX_MESSAGES;
/// Maximum internal messages per v5r1 transaction
pub use ton_transaction::V5R1_MAX_MESSAGES;
/// TON transaction builder
pub use ton_transaction_builder::TonTransactionBuilder;
