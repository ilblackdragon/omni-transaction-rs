//! Minimal required Starknet types.
mod call;
mod data_availability_mode;
mod resource_bounds;

pub use self::call::Call;
pub use self::data_availability_mode::DataAvailabilityMode;
pub use self::resource_bounds::ResourceBounds;

/// A field element of the Stark field (integer in `[0, p)` with
/// `p = 2^251 + 17 * 2^192 + 1`), re-exported from `starknet-types-core`.
///
/// With the `serde` feature enabled, a [`Felt`] (de)serializes as a minimal
/// lowercase hexadecimal string (e.g. `"0x1"`), matching the Starknet RPC `FELT` schema.
pub use starknet_types_core::felt::Felt;

/// A Starknet transaction signature.
///
/// Starknet uses account abstraction, so the signature layout is defined by the sender's
/// account contract: `[r, s]` for Stark-curve accounts (ArgentX / OpenZeppelin default),
/// the Cairo Serde of the account's `Signature` struct for others (e.g. OpenZeppelin
/// `EthAccount` expects `[r_low, r_high, s_low, s_high, y_parity]` for secp256k1).
/// Converting a raw signature into the account-specific felts is the caller's job.
pub type Signature = Vec<Felt>;
