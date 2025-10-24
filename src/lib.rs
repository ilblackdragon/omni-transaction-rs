//! # Omni Transaction
//! `Omni-Transaction` is a library to construct transactions for different chains in Rust clients.
//!
//! The library is designed to be modular and extensible, allowing you to add support for new chains easily.
//!
//! ### Supported chains:
//! - EVM chains (including Ethereum and L2s)
//! - Bitcoin
//!
//! ### Installation
//! ```toml
//! [dependencies]
//! omni-transaction = "0.2.1"
//! ```
//!
//! ### Examples
//!
//! For a complete set of examples see the [examples](https://github.com/Omni-rs/examples.git) repository.
//!
//! ###### Building an Ethereum transaction:
//!
//! ```rust
//! use signet_rs::evm::utils::parse_eth_address;
//! use signet_rs::{TransactionBuilder, TxBuilder, EVM};
//!
//! let to_address_str = "d8dA6BF26964aF9D7eEd9e03E53415D37aA96045";
//! let to_address = parse_eth_address(to_address_str);
//! let max_gas_fee: u128 = 20_000_000_000;
//! let max_priority_fee_per_gas: u128 = 1_000_000_000;
//! let gas_limit: u128 = 21_000;
//! let chain_id: u64 = 1;
//! let nonce: u64 = 0;
//! let data: Vec<u8> = vec![];
//! let value: u128 = 10000000000000000; // 0.01 ETH
//!
//! let evm_tx = TransactionBuilder::new::<EVM>()
//!     .nonce(nonce)
//!     .to(to_address)
//!     .value(value)
//!     .input(data.clone())
//!     .max_priority_fee_per_gas(max_priority_fee_per_gas)
//!     .max_fee_per_gas(max_gas_fee)
//!     .gas_limit(gas_limit)
//!     .chain_id(chain_id)
//!     .build();
//!
//! // Now you have access to build_for_signing that returns the encoded payload
//! let rlp_encoded = evm_tx.build_for_signing();
//! ```
//!
//! ###### Building a Bitcoin transaction:
//!
//! ```rust
//! use signet_rs::bitcoin::types::{
//!     Amount, Hash, LockTime, OutPoint, ScriptBuf, Sequence, TxIn, TxOut, Txid, Version,
//!     Witness, EcdsaSighashType
//! };
//! use signet_rs::{TransactionBuilder, TxBuilder, BITCOIN};
//!
//! let txid_str = "2ece6cd71fee90ff613cee8f30a52c3ecc58685acf9b817b9c467b7ff199871c";
//! let hash = Hash::from_hex(txid_str).unwrap();
//! let txid = Txid(hash);
//! let vout = 0;
//! let txin: TxIn = TxIn {
//!     previous_output: OutPoint::new(txid, vout as u32),
//!     script_sig: ScriptBuf::default(), // For a p2pkh script_sig is initially empty.
//!     sequence: Sequence::MAX,
//!     witness: Witness::default(),
//! };
//! let sender_script_pubkey_hex = "76a914cb8a3018cf279311b148cb8d13728bd8cbe95bda88ac";
//! let sender_script_pubkey = ScriptBuf(sender_script_pubkey_hex.as_bytes().to_vec());
//!
//! let receiver_script_pubkey_hex = "76a914406cf8a18b97a230d15ed82f0d251560a05bda0688ac";
//! let receiver_script_pubkey = ScriptBuf(receiver_script_pubkey_hex.as_bytes().to_vec());
//!
//! // The spend output is locked to a key controlled by the receiver.
//! let spend_txout: TxOut = TxOut {
//!     value: Amount::from_sat(500_000_000),
//!     script_pubkey: receiver_script_pubkey,
//! };
//!
//! let change_txout = TxOut {
//!     value: Amount::from_sat(100_000_000),
//!     script_pubkey: sender_script_pubkey,
//! };
//!
//! let bitcoin_tx = TransactionBuilder::new::<BITCOIN>()
//!     .version(Version::One)
//!     .inputs(vec![txin])
//!     .outputs(vec![spend_txout, change_txout])
//!     .lock_time(LockTime::from_height(0).unwrap())
//!     .build();
//!
//! // Prepare the transaction for signing
//! let encoded_tx = bitcoin_tx.build_for_signing_legacy(EcdsaSighashType::All);
//! ```
//!
//! ### Features
//!
//! - bitcoin
//! - evm
//!
//! By default 'all' the features are enabled. However, you can customize the behaviour like this:
//!
//! ```toml
//! [dependencies]
//! omni-transaction = { version = "0.2.1", features = ["evm"] }
//! ```
//! or
//!
//! ```toml
//! [dependencies]
//! omni-transaction = { version = "0.2.1", features = ["bitcoin"] }
//! ```

// Use no_std except during tests (which need std for test infrastructure)
#![cfg_attr(not(test), no_std)]

extern crate alloc;

#[cfg(feature = "bitcoin")]
pub mod bitcoin;
mod constants;
#[cfg(feature = "evm")]
pub mod evm;
pub mod signer;
mod transaction_builder;
mod transaction_builders;

pub use transaction_builder::{TransactionBuilder, TxBuilder};
/// Alias for BitcoinTransactionBuilder
#[cfg(feature = "bitcoin")]
pub use transaction_builders::BITCOIN;
/// Alias for EVMTransactionBuilder
#[cfg(feature = "evm")]
pub use transaction_builders::EVM;
