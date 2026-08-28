//! Data availability modes for Starknet v3 transactions.
#[cfg(feature = "schemars")]
use schemars::JsonSchema;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Where the state-diff data associated with a field of a v3 transaction is made available.
///
/// Serialized as the strings `"L1"` / `"L2"` in the RPC JSON, and as the integers `0` / `1`
/// inside the packed `data_availability_modes` felt of the transaction hash.
/// Both modes are `L1` on current networks.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub enum DataAvailabilityMode {
    /// Data is posted on L1 (the default and only mode accepted by current networks).
    #[default]
    L1,
    /// Data is kept on L2 (volition; not yet activated on public networks).
    L2,
}

impl DataAvailabilityMode {
    /// The integer value used when packing the mode into the transaction-hash felt
    /// (`L1` = 0, `L2` = 1).
    pub const fn value(self) -> u64 {
        match self {
            Self::L1 => 0,
            Self::L2 => 1,
        }
    }

    /// The RPC string representation of the mode (`"L1"` / `"L2"`).
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::L1 => "L1",
            Self::L2 => "L2",
        }
    }
}
