
pub enum ChainKind {
    NEAR,
    EVM { chain_id: u64 },
    Solana,
    Cosmos { chain_id: String },
}

pub struct OmniAddress {
    chain_kind: ChainKind,
    address: String,
}
