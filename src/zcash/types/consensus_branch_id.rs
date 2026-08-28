#[cfg(feature = "borsh")]
use borsh::{BorshDeserialize, BorshSerialize};
#[cfg(feature = "schemars")]
use schemars::JsonSchema;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// The consensus branch id of the network upgrade epoch a transaction targets.
///
/// The branch id is serialized inside every v5 transaction (`nConsensusBranchId`)
/// and is also mixed into the BLAKE2b personalization of the [ZIP-244] txid and
/// signature digests. A transaction built with the wrong branch id is rejected
/// by consensus, so this value must always be supplied by the caller for the
/// epoch the transaction will be mined in, and re-checked at broadcast time
/// around network upgrades.
///
/// Values are taken from `librustzcash` (`zcash_protocol::consensus::BranchId`).
///
/// [ZIP-244]: https://zips.z.cash/zip-0244
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "borsh", derive(BorshSerialize, BorshDeserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub enum ConsensusBranchId {
    /// NU5, branch id `0xC2D6D0B4` (mainnet activation height 1,687,104).
    Nu5,
    /// NU6, branch id `0xC8E71055`.
    Nu6,
    /// NU6.1, branch id `0x4DEC4DF0`.
    Nu6_1,
    /// NU6.2, branch id `0x5437F330`.
    Nu6_2,
    /// NU6.3 ("Ironwood"), branch id `0x37A5165B` (mainnet activation height 3,428,143).
    Nu6_3,
    /// A custom branch id, for network upgrades not yet known to this library.
    Custom(u32),
}

impl ConsensusBranchId {
    /// Returns the raw `u32` consensus branch id.
    pub const fn to_u32(self) -> u32 {
        match self {
            Self::Nu5 => 0xC2D6_D0B4,
            Self::Nu6 => 0xC8E7_1055,
            Self::Nu6_1 => 0x4DEC_4DF0,
            Self::Nu6_2 => 0x5437_F330,
            Self::Nu6_3 => 0x37A5_165B,
            Self::Custom(id) => id,
        }
    }
}

impl From<ConsensusBranchId> for u32 {
    fn from(branch_id: ConsensusBranchId) -> Self {
        branch_id.to_u32()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_branch_id_values() {
        assert_eq!(ConsensusBranchId::Nu5.to_u32(), 0xC2D6_D0B4);
        assert_eq!(ConsensusBranchId::Nu6.to_u32(), 0xC8E7_1055);
        assert_eq!(ConsensusBranchId::Nu6_1.to_u32(), 0x4DEC_4DF0);
        assert_eq!(ConsensusBranchId::Nu6_2.to_u32(), 0x5437_F330);
        assert_eq!(ConsensusBranchId::Nu6_3.to_u32(), 0x37A5_165B);
        assert_eq!(ConsensusBranchId::Custom(42).to_u32(), 42);
        assert_eq!(u32::from(ConsensusBranchId::Nu5), 0xC2D6_D0B4);
    }
}
