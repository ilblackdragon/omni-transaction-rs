//! Constants for the Aptos module.

/// ASCII salt whose SHA3-256 digest prefixes every `RawTransaction` signing
/// message.
///
/// The signing payload for an Aptos transaction is
/// `sha3_256(APTOS_RAW_TRANSACTION_SALT) || bcs(raw_transaction)`.
/// See `aptos-crypto`'s `HASH_PREFIX` (`b"APTOS::"`) plus the type name
/// `RawTransaction`.
pub const APTOS_RAW_TRANSACTION_SALT: &[u8] = b"APTOS::RawTransaction";
