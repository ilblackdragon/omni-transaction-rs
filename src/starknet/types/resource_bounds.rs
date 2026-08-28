//! Resource bounds for the Starknet v3 fee market ("triple gas" model).
#[cfg(feature = "schemars")]
use schemars::JsonSchema;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// The maximum amount and maximum price per unit of a single resource
/// (`l1_gas`, `l2_gas` or `l1_data_gas`) an INVOKE v3 transaction is allowed to consume.
///
/// Since Starknet 0.13.4 (the "triple gas model", RPC >= 0.8) a broadcasted v3 transaction
/// must carry bounds for all three resources.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct ResourceBounds {
    /// Maximum amount of the resource the transaction may consume.
    ///
    /// Encoded on the wire (both in the transaction hash and in the broadcast JSON) as a `u64`.
    pub max_amount: u64,
    /// Maximum price per unit of the resource (in fri, i.e. 10^-18 STRK) the sender
    /// is willing to pay.
    ///
    /// Encoded on the wire as a `u128`.
    pub max_price_per_unit: u128,
}

impl ResourceBounds {
    /// Creates a new [`ResourceBounds`] from a maximum amount and a maximum price per unit.
    pub const fn new(max_amount: u64, max_price_per_unit: u128) -> Self {
        Self {
            max_amount,
            max_price_per_unit,
        }
    }
}
