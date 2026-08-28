#[cfg(feature = "borsh")]
use borsh::{BorshDeserialize, BorshSerialize};
#[cfg(feature = "schemars")]
use schemars::JsonSchema;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Signature hash type for Zcash transparent inputs ([ZIP-244] S.2a).
///
/// Only these six values are consensus-valid in v5 transactions; any other
/// `hash_type` byte makes the transaction invalid. `SIGHASH_SINGLE` without a
/// corresponding output at the signed input's index is still well defined
/// under ZIP-244 — it commits to the empty output list, rather than hashing
/// the constant `1` as Bitcoin's legacy quirk does.
///
/// [ZIP-244]: https://zips.z.cash/zip-0244
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "borsh", derive(BorshSerialize, BorshDeserialize))]
#[cfg_attr(feature = "borsh", borsh(use_discriminant = true))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[repr(u8)]
pub enum ZcashSighashType {
    /// 0x01: Sign all inputs and outputs.
    All = 0x01,
    /// 0x02: Sign all inputs and no outputs.
    None = 0x02,
    /// 0x03: Sign all inputs and the output with the same index as the signed input.
    Single = 0x03,
    /// 0x81: Sign this input only and all outputs.
    AllPlusAnyoneCanPay = 0x81,
    /// 0x82: Sign this input only and no outputs.
    NonePlusAnyoneCanPay = 0x82,
    /// 0x83: Sign this input only and the output with the same index.
    SinglePlusAnyoneCanPay = 0x83,
}

impl ZcashSighashType {
    /// Returns the raw `hash_type` byte committed to in the signature digest
    /// and appended to the DER signature in the script sig.
    pub const fn to_u8(self) -> u8 {
        self as u8
    }

    /// Returns `true` if the `ANYONECANPAY` flag (bit 0x80) is set.
    pub const fn is_anyone_can_pay(self) -> bool {
        self as u8 & 0x80 != 0
    }

    /// Returns the base hash type (0x01 `ALL`, 0x02 `NONE` or 0x03 `SINGLE`)
    /// with the `ANYONECANPAY` flag masked out.
    pub const fn base_type(self) -> u8 {
        self as u8 & 0x1f
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sighash_type_values() {
        assert_eq!(ZcashSighashType::All.to_u8(), 0x01);
        assert_eq!(ZcashSighashType::None.to_u8(), 0x02);
        assert_eq!(ZcashSighashType::Single.to_u8(), 0x03);
        assert_eq!(ZcashSighashType::AllPlusAnyoneCanPay.to_u8(), 0x81);
        assert_eq!(ZcashSighashType::NonePlusAnyoneCanPay.to_u8(), 0x82);
        assert_eq!(ZcashSighashType::SinglePlusAnyoneCanPay.to_u8(), 0x83);

        assert!(!ZcashSighashType::All.is_anyone_can_pay());
        assert!(ZcashSighashType::AllPlusAnyoneCanPay.is_anyone_can_pay());
        assert_eq!(ZcashSighashType::SinglePlusAnyoneCanPay.base_type(), 0x03);
        assert_eq!(ZcashSighashType::All.base_type(), 0x01);
    }
}
