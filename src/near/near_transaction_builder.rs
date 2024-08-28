use super::{
    near_transaction::NearTransaction,
    types::{Action, PublicKey},
};
use crate::transaction_builder::TxBuilder;

pub struct NearTransactionBuilder {
    pub signer_id: Option<String>,
    pub signer_public_key: Option<PublicKey>,
    pub nonce: Option<u64>,
    pub receiver_id: Option<String>,
    pub block_hash: Option<[u8; 32]>,
    pub actions: Option<Vec<Action>>,
}

impl Default for NearTransactionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl TxBuilder<NearTransaction> for NearTransactionBuilder {
    fn build(&self) -> NearTransaction {
        NearTransaction {
            signer_id: self
                .signer_id
                .clone()
                .expect("Missing sender ID")
                .parse()
                .unwrap(),
            signer_public_key: self
                .signer_public_key
                .clone()
                .expect("Missing signer public key"),
            nonce: self.nonce.expect("Missing nonce"),
            receiver_id: self
                .receiver_id
                .clone()
                .expect("Missing receiver ID")
                .parse()
                .unwrap(),
            block_hash: self.block_hash.expect("Missing block hash"),
            actions: self.actions.clone().expect("Missing actions"),
        }
    }
}

impl NearTransactionBuilder {
    pub const fn new() -> Self {
        Self {
            signer_id: None,
            signer_public_key: None,
            nonce: None,
            receiver_id: None,
            block_hash: None,
            actions: None,
        }
    }

    pub fn signer_id(mut self, signer_id: String) -> Self {
        self.signer_id = Some(signer_id);
        self
    }

    pub const fn signer_public_key(mut self, signer_public_key: PublicKey) -> Self {
        self.signer_public_key = Some(signer_public_key);
        self
    }

    pub const fn nonce(mut self, nonce: u64) -> Self {
        self.nonce = Some(nonce);
        self
    }

    pub fn receiver_id(mut self, receiver_id: String) -> Self {
        self.receiver_id = Some(receiver_id);
        self
    }

    pub const fn block_hash(mut self, block_hash: [u8; 32]) -> Self {
        self.block_hash = Some(block_hash);
        self
    }

    pub fn actions(mut self, actions: Vec<Action>) -> Self {
        self.actions = Some(actions);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::near::types::{
        Action as OmniAction, PublicKey as OmniPublicKey, TransferAction as OmniTransferAction,
    };
    use near_crypto::PublicKey;
    use near_primitives::{
        action::Action, action::TransferAction, hash::CryptoHash, transaction::TransactionV0,
    };

    #[test]
    fn test_near_transaction_builder_against_near_primitives() {
        let signer_id = "alice.near";
        let signer_public_key = [0u8; 64];
        let nonce = 0;
        let receiver_id: &str = "bob.near";
        let block_hash = [0u8; 32];
        let transfer_action = OmniAction::Transfer(OmniTransferAction { deposit: 1u128 });
        let omni_actions = vec![transfer_action];
        let actions = Action::Transfer(TransferAction { deposit: 1u128 });

        let omni_near_transaction = NearTransactionBuilder::new()
            .signer_id(signer_id.to_string())
            .signer_public_key(OmniPublicKey::SECP256K1(signer_public_key.into()))
            .nonce(nonce)
            .receiver_id(receiver_id.to_string())
            .block_hash(block_hash)
            .actions(omni_actions)
            .build();

        let omni_tx_encoded = omni_near_transaction.build_for_signing();

        let v0_tx: TransactionV0 = TransactionV0 {
            signer_id: signer_id.parse().unwrap(),
            public_key: PublicKey::SECP256K1(signer_public_key.into()),
            nonce,
            receiver_id: receiver_id.parse().unwrap(),
            block_hash: CryptoHash([0; 32]),
            actions: vec![actions],
        };

        let serialized_v0_tx = borsh::to_vec(&v0_tx).expect("failed to serialize NEAR transaction");

        assert!(serialized_v0_tx == omni_tx_encoded);
    }
}
