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
        AccessKey as OmniAccessKey, AccessKeyPermission as OmniAccessKeyPermission,
        Action as OmniAction, AddKeyAction as OmniAddKeyAction, PublicKey as OmniPublicKey,
        TransferAction as OmniTransferAction,
    };
    use near_crypto::PublicKey;
    use near_primitives::{
        account::{AccessKey, AccessKeyPermission},
        action::{Action, AddKeyAction, TransferAction},
        hash::CryptoHash,
        transaction::TransactionV0,
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

    #[test]
    fn test_build_for_signing_for_near_against_near_primitives_2() {
        let signer_id = "forgetful-parent.testnet";
        let signer_public_key = "6E8sCci9badyRkXb3JoRpBj5p8C6Tw41ELDZoiihKEtp"; // ed25519:6E8sCci9badyRkXb3JoRpBj5p8C6Tw41ELDZoiihKEtp
        let signer_public_key_as_bytes: [u8; 32] = bs58::decode(signer_public_key)
            .into_vec()
            .expect("Decoding failed")
            .try_into()
            .expect("Invalid length, expected 32 bytes");

        let nonce = 1;
        let receiver_id: &str = "forgetful-parent.testnet";
        let transfer_action = Action::Transfer(TransferAction { deposit: 1u128 });
        let transfer_action_omni = OmniAction::Transfer(OmniTransferAction { deposit: 1u128 });
        let add_key_action_omni = OmniAction::AddKey(Box::new(OmniAddKeyAction {
            public_key: OmniPublicKey::ED25519(signer_public_key_as_bytes.into()),
            access_key: OmniAccessKey {
                nonce: 0,
                permission: OmniAccessKeyPermission::FullAccess,
            },
        }));
        let add_key_action = Action::AddKey(Box::new(AddKeyAction {
            public_key: PublicKey::ED25519(signer_public_key_as_bytes.into()),
            access_key: AccessKey {
                nonce: 0,
                permission: AccessKeyPermission::FullAccess,
            },
        }));

        let block_hash = "4reLvkAWfqk5fsqio1KLudk46cqRz9erQdaHkWZKMJDZ";
        let block_hash_as_bytes: [u8; 32] = bs58::decode(block_hash)
            .into_vec()
            .expect("Decoding failed")
            .try_into()
            .expect("Invalid length, expected 32 bytes");

        let v0_tx: TransactionV0 = TransactionV0 {
            signer_id: signer_id.parse().unwrap(),
            public_key: PublicKey::ED25519(signer_public_key_as_bytes.into()),
            nonce,
            receiver_id: receiver_id.parse().unwrap(),
            block_hash: CryptoHash(block_hash_as_bytes),
            actions: vec![transfer_action, add_key_action],
        };

        let serialized_v0_tx = borsh::to_vec(&v0_tx).expect("failed to serialize NEAR transaction");

        let omni_tx = NearTransaction {
            signer_id: signer_id.parse().unwrap(),
            signer_public_key: OmniPublicKey::ED25519(signer_public_key_as_bytes.into()),
            nonce,
            receiver_id: receiver_id.parse().unwrap(),
            block_hash: block_hash_as_bytes,
            actions: vec![transfer_action_omni, add_key_action_omni],
        };

        let serialized_omni_tx = omni_tx.build_for_signing();

        println!("serialized_v0_tx: {:?}", serialized_v0_tx);
        println!("serialized_omni_tx: {:?}", serialized_omni_tx);

        assert!(serialized_v0_tx == serialized_omni_tx);
    }
}
