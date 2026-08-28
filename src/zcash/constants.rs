//! Constants for the Zcash v5 transaction format ([ZIP-225]).
//!
//! [ZIP-225]: https://zips.z.cash/zip-0225

/// The v5 transaction header field: `fOverwintered` (bit 31) set and version = 5.
///
/// Serialized little-endian as the first four bytes (`05 00 00 80`) of every
/// v5 transaction.
pub const V5_TX_VERSION: u32 = 0x8000_0005;

/// The v5 transaction version group id (Zcash protocol specification, section 7.1.2).
///
/// Serialized little-endian as bytes `0A 27 A7 26`.
pub const V5_VERSION_GROUP_ID: u32 = 0x26A7_270A;

/// Maximum allowed value for `nExpiryHeight` ([ZIP-203]).
///
/// [ZIP-203]: https://zips.z.cash/zip-0203
pub const MAX_EXPIRY_HEIGHT: u32 = 499_999_999;
