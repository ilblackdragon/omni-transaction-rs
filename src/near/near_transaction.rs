use borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{borsh, AccountId};

use super::types::{Action, BlockHash, PublicKey, Signature, U64};

#[derive(Serialize, Deserialize, Debug, Clone, BorshSerialize, BorshDeserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct NearTransaction {
    /// An account on which behalf transaction is signed
    pub signer_id: AccountId,
    /// A public key of the access key which was used to sign an account.
    /// Access key holds permissions for calling certain kinds of actions.
    pub signer_public_key: PublicKey,
    /// Nonce is used to determine order of transaction in the pool.
    /// It increments for a combination of `signer_id` and `public_key`
    pub nonce: U64,
    /// Receiver account for this transaction
    pub receiver_id: AccountId,
    /// The hash of the block in the blockchain on top of which the given transaction is valid
    pub block_hash: BlockHash,
    /// A list of actions to be applied
    pub actions: Vec<Action>,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct SignedTransaction {
    pub transaction: NearTransaction,
    pub signature: Signature,
}

impl NearTransaction {
    pub fn build_for_signing(&self) -> Vec<u8> {
        borsh::to_vec(self).expect("failed to serialize NEAR transaction")
    }

    pub fn build_with_signature(&self, signature: Signature) -> Vec<u8> {
        let signed_tx = SignedTransaction {
            transaction: self.clone(),
            signature,
        };
        borsh::to_vec(&signed_tx).expect("failed to serialize NEAR transaction")
    }

    pub fn from_json(json: &str) -> Result<Self, near_sdk::serde_json::Error> {
        near_sdk::serde_json::from_str(json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::near::types::{
        AccessKey as OmniAccessKey, AccessKeyPermission as OmniAccessKeyPermission,
        Action as OmniAction, AddKeyAction as OmniAddKeyAction,
        CreateAccountAction as OmniCreateAccountAction,
        DeleteAccountAction as OmniDeleteAccountAction, DeleteKeyAction as OmniDeleteKeyAction,
        DeployContractAction as OmniDeployContractAction, ED25519Signature,
        FunctionCallAction as OmniFunctionCallAction, Secp256K1Signature,
        Signature as OmniSignature, StakeAction as OmniStakeAction,
        TransferAction as OmniTransferAction, U128,
    };
    use crate::near::utils::PublicKeyStrExt;
    use near_crypto::{ED25519PublicKey, PublicKey};
    use near_crypto::{InMemorySigner, KeyType, Signer};
    use near_primitives::action::{
        CreateAccountAction, DeleteAccountAction, DeleteKeyAction, DeployContractAction,
        FunctionCallAction, StakeAction,
    };
    use near_primitives::{
        account::{AccessKey, AccessKeyPermission},
        action::{Action, AddKeyAction, TransferAction},
        hash::CryptoHash,
        transaction::Transaction as NearPrimitiveTransaction,
        transaction::TransactionV0,
    };

    #[derive(Debug)]
    struct TestCase {
        signer_id: &'static str,
        signer_public_key: &'static str,
        nonce: u64,
        receiver_id: &'static str,
        block_hash: &'static str,
        near_primitive_actions: Vec<Action>,
        omni_actions: Vec<OmniAction>,
    }

    fn create_test_cases() -> Vec<TestCase> {
        vec![
            // Create Account
            TestCase {
                signer_id: "alice.near",
                signer_public_key: "ed25519:6E8sCci9badyRkXb3JoRpBj5p8C6Tw41ELDZoiihKEtp",
                nonce: 1,
                receiver_id: "bob.near",
                block_hash: "4reLvkAWfqk5fsqio1KLudk46cqRz9erQdaHkWZKMJDZ",
                near_primitive_actions: vec![Action::CreateAccount(CreateAccountAction {})],
                omni_actions: vec![OmniAction::CreateAccount(OmniCreateAccountAction {})],
            },
            // Deploy Contract
            TestCase {
                signer_id: "alice.near",
                signer_public_key: "ed25519:6E8sCci9badyRkXb3JoRpBj5p8C6Tw41ELDZoiihKEtp",
                nonce: 1,
                receiver_id: "bob.near",
                block_hash: "4reLvkAWfqk5fsqio1KLudk46cqRz9erQdaHkWZKMJDZ",
                near_primitive_actions: vec![Action::DeployContract(DeployContractAction {
                    code: vec![0x01, 0x02, 0x03],
                })],
                omni_actions: vec![OmniAction::DeployContract(OmniDeployContractAction {
                    code: vec![0x01, 0x02, 0x03],
                })],
            },
            // Function Call
            TestCase {
                signer_id: "alice.near",
                signer_public_key: "ed25519:6E8sCci9badyRkXb3JoRpBj5p8C6Tw41ELDZoiihKEtp",
                nonce: 1,
                receiver_id: "bob.near",
                block_hash: "4reLvkAWfqk5fsqio1KLudk46cqRz9erQdaHkWZKMJDZ",
                near_primitive_actions: vec![Action::FunctionCall(Box::new(FunctionCallAction {
                    method_name: "function1".to_string(),
                    args: vec![0x01, 0x02, 0x03],
                    gas: 100,
                    deposit: 1u128,
                }))],
                omni_actions: vec![OmniAction::FunctionCall(Box::new(OmniFunctionCallAction {
                    method_name: "function1".to_string(),
                    args: vec![0x01, 0x02, 0x03],
                    gas: U64(100),
                    deposit: U128(1),
                }))],
            },
            // Transfer
            TestCase {
                signer_id: "alice.near",
                signer_public_key: "ed25519:6E8sCci9badyRkXb3JoRpBj5p8C6Tw41ELDZoiihKEtp",
                nonce: 1,
                receiver_id: "bob.near",
                block_hash: "4reLvkAWfqk5fsqio1KLudk46cqRz9erQdaHkWZKMJDZ",
                near_primitive_actions: vec![Action::Transfer(TransferAction { deposit: 1u128 })],
                omni_actions: vec![OmniAction::Transfer(OmniTransferAction {
                    deposit: U128(1),
                })],
            },
            // Stake
            TestCase {
                signer_id: "alice.near",
                signer_public_key: "ed25519:6E8sCci9badyRkXb3JoRpBj5p8C6Tw41ELDZoiihKEtp",
                nonce: 1,
                receiver_id: "bob.near",
                block_hash: "4reLvkAWfqk5fsqio1KLudk46cqRz9erQdaHkWZKMJDZ",
                near_primitive_actions: vec![Action::Stake(Box::new(StakeAction {
                    stake: 1u128,
                    public_key: PublicKey::ED25519(ED25519PublicKey(
                        "ed25519:6E8sCci9badyRkXb3JoRpBj5p8C6Tw41ELDZoiihKEtp"
                            .try_ed25519_into_bytes()
                            .expect("Public key should be 32 bytes"),
                    )),
                }))],
                omni_actions: vec![OmniAction::Stake(Box::new(OmniStakeAction {
                    stake: U128(1),
                    public_key: "ed25519:6E8sCci9badyRkXb3JoRpBj5p8C6Tw41ELDZoiihKEtp"
                        .to_public_key()
                        .unwrap(),
                }))],
            },
            // AddKey
            TestCase {
                signer_id: "alice.near",
                signer_public_key: "ed25519:6E8sCci9badyRkXb3JoRpBj5p8C6Tw41ELDZoiihKEtp",
                nonce: 1,
                receiver_id: "bob.near",
                block_hash: "4reLvkAWfqk5fsqio1KLudk46cqRz9erQdaHkWZKMJDZ",
                near_primitive_actions: vec![Action::AddKey(Box::new(AddKeyAction {
                    public_key: PublicKey::ED25519(ED25519PublicKey(
                        "ed25519:6E8sCci9badyRkXb3JoRpBj5p8C6Tw41ELDZoiihKEtp"
                            .try_ed25519_into_bytes()
                            .expect("Public key should be 32 bytes"),
                    )),
                    access_key: AccessKey {
                        nonce: 0,
                        permission: AccessKeyPermission::FullAccess,
                    },
                }))],
                omni_actions: vec![OmniAction::AddKey(Box::new(OmniAddKeyAction {
                    public_key: "ed25519:6E8sCci9badyRkXb3JoRpBj5p8C6Tw41ELDZoiihKEtp"
                        .to_public_key()
                        .unwrap(),
                    access_key: OmniAccessKey {
                        nonce: U64(0),
                        permission: OmniAccessKeyPermission::FullAccess,
                    },
                }))],
            },
            // DeleteKey
            TestCase {
                signer_id: "alice.near",
                signer_public_key: "ed25519:6E8sCci9badyRkXb3JoRpBj5p8C6Tw41ELDZoiihKEtp",
                nonce: 1,
                receiver_id: "bob.near",
                block_hash: "4reLvkAWfqk5fsqio1KLudk46cqRz9erQdaHkWZKMJDZ",
                near_primitive_actions: vec![Action::DeleteKey(Box::new(DeleteKeyAction {
                    public_key: PublicKey::ED25519(ED25519PublicKey(
                        "ed25519:6E8sCci9badyRkXb3JoRpBj5p8C6Tw41ELDZoiihKEtp"
                            .try_ed25519_into_bytes()
                            .expect("Public key should be 32 bytes"),
                    )),
                }))],
                omni_actions: vec![OmniAction::DeleteKey(Box::new(OmniDeleteKeyAction {
                    public_key: "ed25519:6E8sCci9badyRkXb3JoRpBj5p8C6Tw41ELDZoiihKEtp"
                        .to_public_key()
                        .unwrap(),
                }))],
            },
            // DeleteAccount
            TestCase {
                signer_id: "alice.near",
                signer_public_key: "ed25519:6E8sCci9badyRkXb3JoRpBj5p8C6Tw41ELDZoiihKEtp",
                nonce: 1,
                receiver_id: "bob.near",
                block_hash: "4reLvkAWfqk5fsqio1KLudk46cqRz9erQdaHkWZKMJDZ",
                near_primitive_actions: vec![Action::DeleteAccount(DeleteAccountAction {
                    beneficiary_id: "bob.near".parse().unwrap(),
                })],
                omni_actions: vec![OmniAction::DeleteAccount(OmniDeleteAccountAction {
                    beneficiary_id: "bob.near".parse().unwrap(),
                })],
            },
            // Transfer and Add Key
            TestCase {
                signer_id: "forgetful-parent.testnet",
                signer_public_key: "ed25519:6E8sCci9badyRkXb3JoRpBj5p8C6Tw41ELDZoiihKEtp",
                nonce: 1,
                receiver_id: "forgetful-parent.testnet",
                block_hash: "4reLvkAWfqk5fsqio1KLudk46cqRz9erQdaHkWZKMJDZ",
                near_primitive_actions: vec![
                    Action::Transfer(TransferAction { deposit: 1u128 }),
                    Action::AddKey(Box::new(AddKeyAction {
                        public_key: PublicKey::ED25519(ED25519PublicKey(
                            "ed25519:6E8sCci9badyRkXb3JoRpBj5p8C6Tw41ELDZoiihKEtp"
                                .to_public_key_as_bytes()
                                .unwrap()
                                .try_into()
                                .expect("Public key should be 32 bytes"),
                        )),
                        access_key: AccessKey {
                            nonce: 0,
                            permission: AccessKeyPermission::FullAccess,
                        },
                    })),
                ],
                omni_actions: vec![
                    OmniAction::Transfer(OmniTransferAction { deposit: U128(1) }),
                    OmniAction::AddKey(Box::new(OmniAddKeyAction {
                        public_key: "ed25519:6E8sCci9badyRkXb3JoRpBj5p8C6Tw41ELDZoiihKEtp"
                            .to_public_key()
                            .unwrap(),
                        access_key: OmniAccessKey {
                            nonce: U64(0),
                            permission: OmniAccessKeyPermission::FullAccess,
                        },
                    })),
                ],
            },
        ]
    }

    #[test]
    fn test_build_for_signing_for_near_against_near_primitives() {
        let test_cases = create_test_cases();

        for (i, test_case) in test_cases.iter().enumerate() {
            let near_primitive_v0_tx: TransactionV0 = TransactionV0 {
                signer_id: test_case.signer_id.parse().unwrap(),
                public_key: PublicKey::ED25519(ED25519PublicKey(
                    test_case
                        .signer_public_key
                        .to_public_key_as_bytes()
                        .unwrap()
                        .try_into()
                        .expect("Public key should be 32 bytes"),
                )),
                nonce: test_case.nonce,
                receiver_id: test_case.receiver_id.parse().unwrap(),
                block_hash: CryptoHash(test_case.block_hash.to_fixed_32_bytes().unwrap()),
                actions: test_case.near_primitive_actions.clone(),
            };

            let serialized_near_primitive_v0_tx =
                borsh::to_vec(&near_primitive_v0_tx).expect("failed to serialize NEAR transaction");

            let omni_tx = NearTransaction {
                signer_id: test_case.signer_id.parse().unwrap(),
                signer_public_key: test_case.signer_public_key.to_public_key().unwrap(),
                nonce: U64(test_case.nonce),
                receiver_id: test_case.receiver_id.parse().unwrap(),
                block_hash: test_case.block_hash.to_block_hash().unwrap(),
                actions: test_case.omni_actions.clone(),
            };

            let serialized_omni_tx = omni_tx.build_for_signing();

            assert_eq!(
                serialized_near_primitive_v0_tx, serialized_omni_tx,
                "Test case {} failed: serialized transactions do not match.\nNEAR: {:?}\nOmni: {:?}",
                i, serialized_near_primitive_v0_tx, serialized_omni_tx
            );
        }
    }

    #[test]
    fn test_build_with_signature_against_near_primitives_for_ed25519() {
        let test_cases = create_test_cases();

        for (i, test_case) in test_cases.iter().enumerate() {
            let near_primitive_v0_tx: TransactionV0 = TransactionV0 {
                signer_id: test_case.signer_id.parse().unwrap(),
                public_key: PublicKey::ED25519(ED25519PublicKey(
                    test_case
                        .signer_public_key
                        .to_public_key_as_bytes()
                        .unwrap()
                        .try_into()
                        .expect("Public key should be 32 bytes"),
                )),
                nonce: test_case.nonce,
                receiver_id: test_case.receiver_id.parse().unwrap(),
                block_hash: CryptoHash(test_case.block_hash.to_fixed_32_bytes().unwrap()),
                actions: test_case.near_primitive_actions.clone(),
            };

            let near_primitive_tx = NearPrimitiveTransaction::V0(near_primitive_v0_tx);

            let in_memory_signer =
                InMemorySigner::from_random("account_id".parse().unwrap(), KeyType::ED25519);

            let signer = Signer::InMemory(in_memory_signer);

            let signed_tx = near_primitive_tx.sign(&signer);

            let encoded_signed_tx =
                borsh::to_vec(&signed_tx).expect("failed to serialize signed transaction");

            let omni_tx = NearTransaction {
                signer_id: test_case.signer_id.parse().unwrap(),
                signer_public_key: test_case.signer_public_key.to_public_key().unwrap(),
                nonce: U64(test_case.nonce),
                receiver_id: test_case.receiver_id.parse().unwrap(),
                block_hash: test_case.block_hash.to_block_hash().unwrap(),
                actions: test_case.omni_actions.clone(),
            };

            // @dev For testing purposes, we are only supporting ED25519 signatures
            let signature_bytes: [u8; 64] = match &signed_tx.signature {
                near_crypto::Signature::ED25519(sig) => sig.to_bytes(),
                _ => panic!("Unsupported signature type"),
            };

            let omni_signature = OmniSignature::ED25519(ED25519Signature {
                r: signature_bytes[..32].try_into().unwrap(),
                s: signature_bytes[32..].try_into().unwrap(),
            });

            let serialized_omni_tx = omni_tx.build_with_signature(omni_signature);

            assert_eq!(
                serialized_omni_tx, encoded_signed_tx,
                "Test case {} failed: serialized transactions do not match.\nNEAR: {:?}\nOmni: {:?}",
                i, serialized_omni_tx, encoded_signed_tx
            );
        }
    }

    #[test]
    fn test_build_with_signature_against_near_primitives_for_secp256k1() {
        let test_cases = create_test_cases();

        for (i, test_case) in test_cases.iter().enumerate() {
            let near_primitive_v0_tx: TransactionV0 = TransactionV0 {
                signer_id: test_case.signer_id.parse().unwrap(),
                public_key: PublicKey::ED25519(ED25519PublicKey(
                    test_case
                        .signer_public_key
                        .to_public_key_as_bytes()
                        .unwrap()
                        .try_into()
                        .expect("Public key should be 32 bytes"),
                )),
                nonce: test_case.nonce,
                receiver_id: test_case.receiver_id.parse().unwrap(),
                block_hash: CryptoHash(test_case.block_hash.to_fixed_32_bytes().unwrap()),
                actions: test_case.near_primitive_actions.clone(),
            };

            let near_primitive_tx = NearPrimitiveTransaction::V0(near_primitive_v0_tx);

            let in_memory_signer =
                InMemorySigner::from_random("account_id".parse().unwrap(), KeyType::SECP256K1);

            let signer = Signer::InMemory(in_memory_signer);

            let signed_tx = near_primitive_tx.sign(&signer);

            let encoded_signed_tx =
                borsh::to_vec(&signed_tx).expect("failed to serialize signed transaction");

            let omni_tx = NearTransaction {
                signer_id: test_case.signer_id.parse().unwrap(),
                signer_public_key: test_case.signer_public_key.to_public_key().unwrap(),
                nonce: U64(test_case.nonce),
                receiver_id: test_case.receiver_id.parse().unwrap(),
                block_hash: test_case.block_hash.to_block_hash().unwrap(),
                actions: test_case.omni_actions.clone(),
            };

            // @dev For testing purposes, we are only supporting SECP256K1 signatures
            let signature_serialized = serde_json::to_string(&signed_tx.signature).unwrap();

            // Extract the actual signature bytes from the serialized string
            let signature_str = signature_serialized
                .trim_matches('"')
                .split(':')
                .nth(1)
                .unwrap();

            let signature_str = signature_str.trim();

            // Decode the signature from Base58
            let signature_bytes = bs58::decode(signature_str)
                .into_vec()
                .expect("Failed to decode Base58 signature");

            let mut signature_array: [u8; 65] = [0; 65];
            signature_array.copy_from_slice(&signature_bytes);

            let omni_signature = OmniSignature::SECP256K1(Secp256K1Signature(signature_array));

            let serialized_omni_tx = omni_tx.build_with_signature(omni_signature);

            assert_eq!(
                serialized_omni_tx, encoded_signed_tx,
                "Test case {} failed: serialized transactions do not match.\nNEAR: {:?}\nOmni: {:?}",
                i, serialized_omni_tx, encoded_signed_tx
            );
        }
    }

    #[test]
    fn test_build_for_signing_for_omni_against_near_primitives_using_json_input() {
        let input = r#"
        {
            "signer_id": "86a315fdc1c4211787aa2fd78a50041ee581c7fff6cec2535ebec14af5c40381",
            "signer_public_key": "ed25519:A4ZsCYMqJ1oHFGR2g2mFrwhQvaWmyz8K5c5FvfxEPF52",
            "nonce": 172237399000001,
            "receiver_id": "86a315fdc1c4211787aa2fd78a50041ee581c7fff6cec2535ebec14af5c40381",
            "block_hash": "4reLvkAWfqk5fsqio1KLudk46cqRz9erQdaHkWZKMJDZ",
            "actions": [
                { "Transfer": { "deposit": "1" } },
                {
                    "AddKey": {
                        "public_key": "ed25519:6E8sCci9badyRkXb3JoRpBj5p8C6Tw41ELDZoiihKEtp",
                        "access_key": { "nonce": "0", "permission": "FullAccess" }
                    }
                },
                {
                    "DeleteKey": {
                        "public_key": "ed25519:6E8sCci9badyRkXb3JoRpBj5p8C6Tw41ELDZoiihKEtp"
                    }
                }
            ]
        }
    "#;

        let tx = NearTransaction::from_json(input).unwrap();

        assert!(tx.signer_id == "86a315fdc1c4211787aa2fd78a50041ee581c7fff6cec2535ebec14af5c40381");
        assert!(
            tx.signer_public_key
                == "ed25519:A4ZsCYMqJ1oHFGR2g2mFrwhQvaWmyz8K5c5FvfxEPF52"
                    .to_public_key()
                    .unwrap()
        );
        assert!(tx.nonce == U64(172237399000001));
        assert!(
            tx.receiver_id == "86a315fdc1c4211787aa2fd78a50041ee581c7fff6cec2535ebec14af5c40381"
        );
        assert!(
            tx.block_hash
                == "4reLvkAWfqk5fsqio1KLudk46cqRz9erQdaHkWZKMJDZ"
                    .to_block_hash()
                    .unwrap()
        );
        assert!(tx.actions.len() == 3);
    }

    #[test]
    fn test_from_json() {
        let input = r#"
        {
            "signer_id": "forgetful-parent.testnet",
            "signer_public_key": "ed25519:6E8sCci9badyRkXb3JoRpBj5p8C6Tw41ELDZoiihKEtp",
            "nonce": 1,
            "receiver_id": "forgetful-parent.testnet",
            "block_hash": "4reLvkAWfqk5fsqio1KLudk46cqRz9erQdaHkWZKMJDZ",
            "actions": [
                { "Transfer": { "deposit": 1 } }
            ]
        }"#;

        let tx = NearTransaction::from_json(input).unwrap();

        assert!(tx.signer_id == "forgetful-parent.testnet");
        assert!(
            tx.signer_public_key
                == "ed25519:6E8sCci9badyRkXb3JoRpBj5p8C6Tw41ELDZoiihKEtp"
                    .to_public_key()
                    .unwrap()
        );
        assert!(tx.nonce == U64(1));
        assert!(tx.receiver_id == "forgetful-parent.testnet");
        assert!(
            tx.block_hash
                == "4reLvkAWfqk5fsqio1KLudk46cqRz9erQdaHkWZKMJDZ"
                    .to_block_hash()
                    .unwrap()
        );
        assert!(tx.actions.len() == 1);
    }
}
