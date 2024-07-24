
use crate::types::ChainKind;

// Multichain transaction builder.
pub struct TransactionBuilder {
    receiver_id: Option<String>,
    amount: Option<u128>,
    bytecode: Option<Vec<u8>>,
    gas_price: Option<u128>,
    gas_limit: Option<u128>,
}

impl TransactionBuilder {
    pub fn new() -> Self {
        Self {
            receiver_id: None,
            amount: None,
            bytecode: None,
            gas_price: None,
            gas_limit: None,
        }
    }

    /// Recevier of the transaction.
    pub fn receiver(mut self, receiver_id: String) -> Self {
        self.receiver_id = Some(receiver_id);
        self
    }

    /// Amount attached to the transaction.
    pub fn amount(mut self, amount: u128) -> Self {
        self.amount = Some(amount);
        self
    }

    /// Deploy contract with the given bytecode.
    pub fn deploy_contract(mut self, bytecode: &[u8]) -> Self {
        self.bytecode = Some(bytecode.to_vec());
        self
    }

    pub fn gas_price(mut self, gas_price: u128) -> Self {
        self.gas_price = Some(gas_price);
        self
    }

    pub fn gas_limit(mut self, gas_limit: u128) -> Self {
        self.gas_limit = Some(gas_limit);
        self
    }

    /// Build a transaction for the given chain into serialized payload.
    pub fn build(self, chain_kind: ChainKind) -> Vec<u8> {
        // Build a transaction
        match chain_kind {
            ChainKind::NEAR => {
                // Build a NEAR transaction
                vec![]
            }
            ChainKind::EVM { chain_id } => {
                // Build an EVM transaction
                vec![]
            }
            ChainKind::Solana => {
                // Build a Solana transaction
                vec![]
            }
            ChainKind::Cosmos { chain_id } => {
                // Build a Cosmos transaction
                vec![]
            }
        }
     }
}
