use borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::serde::{Deserialize, Serialize};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, BorshSerialize, BorshDeserialize,
)]
#[borsh(use_discriminant = true)]
pub enum EcdsaSighashType {
    /// 0x1: Sign all outputs.
    All = 0x01,
    /// 0x2: Sign no outputs --- anyone can choose the destination.
    None = 0x02,
    /// 0x3: Sign the output whose index matches this input's index. If none exists,
    /// sign the hash `0000000000000000000000000000000000000000000000000000000000000001`.
    /// (This rule is probably an unintentional C++ism, but it's consensus so we have
    /// to follow it.)
    Single = 0x03,
    /// 0x81: Sign all outputs but only this input.
    AllPlusAnyoneCanPay = 0x81,
    /// 0x82: Sign no outputs and only this input.
    NonePlusAnyoneCanPay = 0x82,
    /// 0x83: Sign one output and only this input (see `Single` for what "one output" means).
    SinglePlusAnyoneCanPay = 0x83,
}

impl From<EcdsaSighashType> for u32 {
    fn from(sighash_type: EcdsaSighashType) -> Self {
        sighash_type as u32
    }
}
