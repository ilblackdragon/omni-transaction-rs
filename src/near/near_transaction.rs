use borsh::BorshSerialize;
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{borsh, AccountId};

use super::types::{Action, PublicKey};

#[derive(Serialize, Deserialize, Debug, Clone, BorshSerialize)]
#[serde(crate = "near_sdk::serde")]
pub struct NearTransaction {
    /// An account on which behalf transaction is signed
    pub signer_id: AccountId,
    /// A public key of the access key which was used to sign an account.
    /// Access key holds permissions for calling certain kinds of actions.
    pub signer_public_key: PublicKey,
    /// Nonce is used to determine order of transaction in the pool.
    /// It increments for a combination of `signer_id` and `public_key`
    pub nonce: u64,
    /// Receiver account for this transaction
    pub receiver_id: AccountId,
    /// The hash of the block in the blockchain on top of which the given transaction is valid
    pub block_hash: [u8; 32],
    /// A list of actions to be applied
    pub actions: Vec<Action>,
}

impl NearTransaction {
    pub fn build_for_signing(&self) -> Vec<u8> {
        let tx = Self {
            signer_id: self.signer_id.clone(),
            signer_public_key: self.signer_public_key.clone(),
            nonce: self.nonce,
            receiver_id: self.receiver_id.clone(),
            block_hash: self.block_hash,
            actions: self.actions.clone(),
        };
        borsh::to_vec(&tx).expect("failed to serialize NEAR transaction")
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
    fn test_build_for_signing_for_near_against_near_primitives() {
        let signer_id = "alice.near";
        let signer_public_key = [0u8; 64];
        let nonce = 0;
        let receiver_id: &str = "bob.near";
        let actions = Action::Transfer(TransferAction { deposit: 1u128 });
        let omni_actions = OmniAction::Transfer(OmniTransferAction { deposit: 1u128 });

        let v0_tx: TransactionV0 = TransactionV0 {
            signer_id: signer_id.parse().unwrap(),
            public_key: PublicKey::SECP256K1(signer_public_key.into()),
            nonce,
            receiver_id: receiver_id.parse().unwrap(),
            block_hash: CryptoHash([0; 32]),
            actions: vec![actions],
        };

        let serialized_v0_tx = borsh::to_vec(&v0_tx).expect("failed to serialize NEAR transaction");

        let omni_tx = NearTransaction {
            signer_id: signer_id.parse().unwrap(),
            signer_public_key: OmniPublicKey::SECP256K1(signer_public_key.into()),
            nonce,
            receiver_id: receiver_id.parse().unwrap(),
            block_hash: [0u8; 32],
            actions: vec![omni_actions],
        };

        let serialized_omni_tx = omni_tx.build_for_signing();

        assert!(serialized_v0_tx == serialized_omni_tx);
    }
}
