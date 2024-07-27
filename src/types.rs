use near_sdk::serde::{Serialize, Deserialize};
use near_sdk::borsh::{BorshDeserialize, BorshSerialize};

#[derive(Serialize, Deserialize, BorshDeserialize, BorshSerialize, Clone, PartialEq, Eq, PartialOrd, Hash)]
#[serde(crate = "near_sdk::serde")]
pub enum ChainKind {
    NEAR,
    EVM { chain_id: u64 },
    Solana,
    Cosmos { chain_id: String },
}

#[derive(Serialize, Deserialize, BorshDeserialize, BorshSerialize, PartialEq, Eq)]
#[serde(crate = "near_sdk::serde")]
pub struct OmniAddress {
    pub chain_kind: ChainKind,
    pub address: String,
}
