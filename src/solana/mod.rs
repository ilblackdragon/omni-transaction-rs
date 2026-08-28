//! Transaction builder, encoders, types and utilities for Solana (and other
//! SVM chains such as Fogo or Eclipse, which use the identical wire format
//! and JSON-RPC interface).
//!
//! The builder compiles ordinary [`types::Instruction`]s into a legacy
//! message, or into a versioned V0 message when
//! [`SolanaTransactionBuilder::address_lookup_tables`] is supplied,
//! byte-matching the official Solana SDK compiler.
//!
//! Unlike the secp256k1-based chains in this crate, Solana transaction
//! signatures are **ed25519 over the raw serialized message bytes**:
//! [`SolanaTransaction::build_for_signing`] returns the exact signing
//! preimage (never a hash), which is what a NEAR MPC `Eddsa` sign request
//! must carry, and the fee payer must be the ed25519 public key that will
//! produce the signature.
//!
//! ###### Example:
//!
//! ```rust
//! use omni_transaction::solana::types::{Blockhash, SolanaAddress, SolanaSignature};
//! use omni_transaction::solana::utils::system_transfer;
//! use omni_transaction::{TransactionBuilder, TxBuilder, SOLANA};
//!
//! let payer = SolanaAddress::from_base58("AKnL4NNf3DGWZJS6cPknBuEGnVsV4A4m5tgebLHaRSZ9").unwrap();
//! let to = SolanaAddress::from_base58("8opHzTAnfzRpPEx21XtnrVTX28YQuCpAjcn1PczScKh").unwrap();
//! let recent_blockhash =
//!     Blockhash::from_base58("EETubP5AKHgjPAhzPAFcb8BAY1hMH639CWCFTqi3hq1k").unwrap();
//!
//! let solana_tx = TransactionBuilder::new::<SOLANA>()
//!     .payer(payer)
//!     .instructions(vec![system_transfer(payer, to, 42)])
//!     .recent_blockhash(recent_blockhash)
//!     .build();
//!
//! // The ed25519 signing payload: the serialized message bytes (a preimage,
//! // never a hash). Pass these bytes to the signer (e.g. NEAR MPC `Eddsa`).
//! let payload = solana_tx.build_for_signing();
//!
//! // ...obtain the 64-byte ed25519 signature of `payload`...
//! let signature = SolanaSignature([0u8; 64]);
//!
//! // The broadcastable wire bytes; base64-encode them for JSON-RPC
//! // `sendTransaction` with `{"encoding": "base64"}`.
//! let signed_tx = solana_tx.build_with_signature(&[signature]);
//! ```
pub mod constants;
pub(crate) mod encoding;
mod solana_transaction;
mod solana_transaction_builder;
pub mod types;
pub mod utils;

/// Solana transaction
pub use solana_transaction::SolanaTransaction;
/// Solana transaction builder
pub use solana_transaction_builder::SolanaTransactionBuilder;
