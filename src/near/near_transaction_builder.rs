//! Transaction builder for NEAR transactions
use super::{
    near_transaction::{NearTransaction, TransactionV0, TransactionV1},
    types::{Action, BlockHash, PublicKey},
};
use crate::transaction_builder::TxBuilder;

pub struct NearTransactionBuilder {
    pub signer_id: Option<String>,
    pub signer_public_key: Option<PublicKey>,
    pub nonce: Option<u64>,
    pub receiver_id: Option<String>,
    pub block_hash: Option<BlockHash>,
    pub actions: Option<Vec<Action>>,
    pub priority_fee: Option<u64>,
}

impl Default for NearTransactionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl TxBuilder<NearTransaction> for NearTransactionBuilder {
    fn build(&self) -> NearTransaction {
        let signer_id = self
            .signer_id
            .clone()
            .expect("Missing sender ID")
            .parse()
            .unwrap();
        let signer_public_key = self
            .signer_public_key
            .clone()
            .expect("Missing signer public key");
        let nonce = self.nonce.expect("Missing nonce").into();
        let receiver_id = self
            .receiver_id
            .clone()
            .expect("Missing receiver ID")
            .parse()
            .unwrap();
        let block_hash = self.block_hash.clone().expect("Missing block hash");
        let actions = self.actions.clone().expect("Missing actions");

        match self.priority_fee {
            Some(priority_fee) => NearTransaction::V1(TransactionV1 {
                signer_id,
                signer_public_key,
                nonce,
                receiver_id,
                block_hash,
                actions,
                priority_fee,
            }),
            None => NearTransaction::V0(TransactionV0 {
                signer_id,
                signer_public_key,
                nonce,
                receiver_id,
                block_hash,
                actions,
            }),
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
            priority_fee: None,
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

    pub const fn block_hash(mut self, block_hash: BlockHash) -> Self {
        self.block_hash = Some(block_hash);
        self
    }

    pub fn actions(mut self, actions: Vec<Action>) -> Self {
        self.actions = Some(actions);
        self
    }

    pub const fn priority_fee(mut self, priority_fee: u64) -> Self {
        self.priority_fee = Some(priority_fee);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::near::types::{
        Action as OmniAction, PublicKey as OmniPublicKey, TransferAction as OmniTransferAction,
        U128,
    };
    use near_crypto::PublicKey;
    use near_primitives::{
        action::Action, action::TransferAction, hash::CryptoHash, transaction::TransactionV0,
        types::Balance,
    };

    #[test]
    fn test_near_transaction_builder_against_near_primitives() {
        let signer_id = "alice.near";
        let signer_public_key = [0u8; 64];
        let nonce = 0;
        let receiver_id: &str = "bob.near";
        let block_hash = BlockHash([0u8; 32]);
        let transfer_action = OmniAction::Transfer(OmniTransferAction { deposit: U128(1) });
        let omni_actions = vec![transfer_action];
        let actions = Action::Transfer(TransferAction {
            deposit: Balance::from_yoctonear(1),
        });

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

    #[test]
    fn test_near_transaction_builder_with_priority_fee() {
        use near_primitives::{
            transaction::Transaction as NearPrimitiveTransaction, transaction::TransactionV1,
        };

        let signer_id = "alice.near";
        let signer_public_key = [0u8; 64];
        let nonce = 0;
        let receiver_id: &str = "bob.near";
        let block_hash = BlockHash([0u8; 32]);
        let priority_fee = 1000u64;
        let transfer_action = OmniAction::Transfer(OmniTransferAction { deposit: U128(1) });
        let omni_actions = vec![transfer_action];
        let actions = Action::Transfer(TransferAction {
            deposit: Balance::from_yoctonear(1),
        });

        // Build with priority fee
        let omni_near_transaction = NearTransactionBuilder::new()
            .signer_id(signer_id.to_string())
            .signer_public_key(OmniPublicKey::SECP256K1(signer_public_key.into()))
            .nonce(nonce)
            .receiver_id(receiver_id.to_string())
            .block_hash(block_hash)
            .actions(omni_actions)
            .priority_fee(priority_fee)
            .build();

        // Verify it's a V1 transaction
        assert!(matches!(omni_near_transaction, NearTransaction::V1(_)));
        assert_eq!(omni_near_transaction.priority_fee(), Some(priority_fee));

        let omni_tx_encoded = omni_near_transaction.build_for_signing();

        let v1_tx = TransactionV1 {
            signer_id: signer_id.parse().unwrap(),
            public_key: PublicKey::SECP256K1(signer_public_key.into()),
            nonce,
            receiver_id: receiver_id.parse().unwrap(),
            block_hash: CryptoHash([0; 32]),
            actions: vec![actions],
            priority_fee,
        };

        let near_primitive_tx = NearPrimitiveTransaction::V1(v1_tx);
        let serialized_v1_tx =
            borsh::to_vec(&near_primitive_tx).expect("failed to serialize NEAR transaction");

        assert_eq!(serialized_v1_tx, omni_tx_encoded);
    }

    #[test]
    fn test_near_transaction_builder_without_priority_fee_creates_v0() {
        // Build without priority fee
        let omni_near_transaction = NearTransactionBuilder::new()
            .signer_id("alice.near".to_string())
            .signer_public_key(OmniPublicKey::SECP256K1([0u8; 64].into()))
            .nonce(0)
            .receiver_id("bob.near".to_string())
            .block_hash(BlockHash([0u8; 32]))
            .actions(vec![OmniAction::Transfer(OmniTransferAction {
                deposit: U128(1),
            })])
            .build();

        // Verify it's a V0 transaction
        assert!(matches!(omni_near_transaction, NearTransaction::V0(_)));
        assert_eq!(omni_near_transaction.priority_fee(), None);
    }
}
