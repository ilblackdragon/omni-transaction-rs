#[cfg(feature = "borsh")]
use borsh::{BorshDeserialize, BorshSerialize};
use near_account_id::AccountId;
#[cfg(feature = "schemars")]
use schemars::JsonSchema;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "borsh")]
use std::io::{Error, ErrorKind, Read, Write};

use super::types::{Action, BlockHash, PublicKey, Signature, U64};

/// NEAR transaction V0 (without priority fee)
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "borsh", derive(BorshSerialize, BorshDeserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct TransactionV0 {
    /// An account on which behalf transaction is signed
    pub signer_id: AccountId,
    /// A public key of the access key which was used to sign an account.
    /// Access key holds permissions for calling certain kinds of actions.
    #[cfg_attr(feature = "serde", serde(rename = "public_key"))]
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

/// NEAR transaction V1 (with priority fee)
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "borsh", derive(BorshSerialize, BorshDeserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct TransactionV1 {
    /// An account on which behalf transaction is signed
    pub signer_id: AccountId,
    /// A public key of the access key which was used to sign an account.
    /// Access key holds permissions for calling certain kinds of actions.
    #[cfg_attr(feature = "serde", serde(rename = "public_key"))]
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
    /// Priority fee. Unit is 10^12 yoctoNEAR
    pub priority_fee: u64,
}

/// NEAR transaction abstraction supporting both V0 and V1
///
/// ###### Example (V0 - without priority fee):
/// ```rust
/// use omni_transaction::near::utils::PublicKeyStrExt;
/// use omni_transaction::near::types::{Action, TransferAction, U64, U128};
/// use omni_transaction::near::{NearTransaction, TransactionV0};
///
/// let signer_id = "alice.near";
/// let signer_public_key = "ed25519:6E8sCci9badyRkXb3JoRpBj5p8C6Tw41ELDZoiihKEtp";
/// let nonce = U64(0);
/// let receiver_id = "bob.near";
/// let block_hash_str = "4reLvkAWfqk5fsqio1KLudk46cqRz9erQdaHkWZKMJDZ";
/// let transfer_action = Action::Transfer(TransferAction { deposit: U128(1) });
/// let actions = vec![transfer_action];
///
/// let omni_tx = NearTransaction::V0(TransactionV0 {
///     signer_id: signer_id.parse().unwrap(),
///     signer_public_key: signer_public_key.to_public_key().unwrap(),
///     nonce,
///     receiver_id: receiver_id.parse().unwrap(),
///     block_hash: block_hash_str.to_block_hash().unwrap(),
///     actions,
/// });
/// ```
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub enum NearTransaction {
    V0(TransactionV0),
    V1(TransactionV1),
}

// Custom Borsh serialization for NearTransaction to maintain backwards compatibility
#[cfg(feature = "borsh")]
impl BorshSerialize for NearTransaction {
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<(), Error> {
        match self {
            Self::V0(tx) => BorshSerialize::serialize(tx, writer)?,
            Self::V1(tx) => {
                BorshSerialize::serialize(&1_u8, writer)?;
                BorshSerialize::serialize(tx, writer)?;
            }
        }
        Ok(())
    }
}

// Custom Borsh deserialization for NearTransaction to maintain backwards compatibility
#[cfg(feature = "borsh")]
impl BorshDeserialize for NearTransaction {
    /// Deserialize based on the first and second bytes of the stream. For V0, we do backward compatible deserialization by deserializing
    /// the entire stream into V0. For V1, we consume the first byte and then deserialize the rest.
    fn deserialize_reader<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        let u1 = u8::deserialize_reader(reader)?;
        let u2 = u8::deserialize_reader(reader)?;
        let u3 = u8::deserialize_reader(reader)?;
        let u4 = u8::deserialize_reader(reader)?;
        // This is a ridiculous hackery: because the first field in `TransactionV0` is an `AccountId`
        // and an account id is at most 64 bytes, for all valid `TransactionV0` the second byte must be 0
        // because of the little endian encoding of the length of the account id.
        // On the other hand, for `TransactionV1`, since the first byte is 1 and an account id must have nonzero
        // length, so the second byte must not be zero. Therefore, we can distinguish between the two versions
        // by looking at the second byte.

        let read_signer_id = |buf: [u8; 4], reader: &mut R| -> std::io::Result<AccountId> {
            let str_len = u32::from_le_bytes(buf);
            // AccountId max length is 64 bytes - validate to prevent memory exhaustion
            if str_len > 64 {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    "AccountId length exceeds maximum of 64 bytes",
                ));
            }
            let mut str_vec = Vec::with_capacity(str_len as usize);
            for _ in 0..str_len {
                str_vec.push(u8::deserialize_reader(reader)?);
            }
            AccountId::try_from(String::from_utf8(str_vec).map_err(|_| {
                Error::new(
                    ErrorKind::InvalidData,
                    "Failed to parse AccountId from bytes",
                )
            })?)
            .map_err(|e| Error::new(ErrorKind::InvalidData, e.to_string()))
        };

        if u2 == 0 {
            let signer_id = read_signer_id([u1, u2, u3, u4], reader)?;
            let signer_public_key = PublicKey::deserialize_reader(reader)?;
            let nonce = U64::deserialize_reader(reader)?;
            let receiver_id = AccountId::deserialize_reader(reader)?;
            let block_hash = BlockHash::deserialize_reader(reader)?;
            let actions = Vec::<Action>::deserialize_reader(reader)?;
            Ok(Self::V0(TransactionV0 {
                signer_id,
                signer_public_key,
                nonce,
                receiver_id,
                block_hash,
                actions,
            }))
        } else {
            let u5 = u8::deserialize_reader(reader)?;
            let signer_id = read_signer_id([u2, u3, u4, u5], reader)?;
            let signer_public_key = PublicKey::deserialize_reader(reader)?;
            let nonce = U64::deserialize_reader(reader)?;
            let receiver_id = AccountId::deserialize_reader(reader)?;
            let block_hash = BlockHash::deserialize_reader(reader)?;
            let actions = Vec::<Action>::deserialize_reader(reader)?;
            let priority_fee = u64::deserialize_reader(reader)?;
            Ok(Self::V1(TransactionV1 {
                signer_id,
                signer_public_key,
                nonce,
                receiver_id,
                block_hash,
                actions,
                priority_fee,
            }))
        }
    }
}

/// Signed NEAR transaction abstraction
#[derive(Debug, Clone)]
#[cfg_attr(feature = "borsh", derive(BorshSerialize, BorshDeserialize))]
pub struct SignedTransaction {
    pub transaction: NearTransaction,
    pub signature: Signature,
}

impl NearTransaction {
    /// Get the signer_id for this transaction
    pub const fn signer_id(&self) -> &AccountId {
        match self {
            Self::V0(tx) => &tx.signer_id,
            Self::V1(tx) => &tx.signer_id,
        }
    }

    /// Get the receiver_id for this transaction
    pub const fn receiver_id(&self) -> &AccountId {
        match self {
            Self::V0(tx) => &tx.receiver_id,
            Self::V1(tx) => &tx.receiver_id,
        }
    }

    /// Get the signer_public_key for this transaction
    pub const fn signer_public_key(&self) -> &PublicKey {
        match self {
            Self::V0(tx) => &tx.signer_public_key,
            Self::V1(tx) => &tx.signer_public_key,
        }
    }

    /// Get the nonce for this transaction
    pub const fn nonce(&self) -> U64 {
        match self {
            Self::V0(tx) => tx.nonce,
            Self::V1(tx) => tx.nonce,
        }
    }

    /// Get the actions for this transaction
    pub fn actions(&self) -> &[Action] {
        match self {
            Self::V0(tx) => &tx.actions,
            Self::V1(tx) => &tx.actions,
        }
    }

    /// Get the block_hash for this transaction
    pub const fn block_hash(&self) -> &BlockHash {
        match self {
            Self::V0(tx) => &tx.block_hash,
            Self::V1(tx) => &tx.block_hash,
        }
    }

    /// Get the priority_fee for this transaction (None for V0, Some for V1)
    pub const fn priority_fee(&self) -> Option<u64> {
        match self {
            Self::V0(_) => None,
            Self::V1(tx) => Some(tx.priority_fee),
        }
    }

    #[cfg(feature = "borsh")]
    pub fn build_for_signing(&self) -> Vec<u8> {
        borsh::to_vec(self).expect("failed to serialize NEAR transaction")
    }

    #[cfg(feature = "borsh")]
    pub fn build_with_signature(&self, signature: Signature) -> Vec<u8> {
        let signed_tx = SignedTransaction {
            transaction: self.clone(),
            signature,
        };
        borsh::to_vec(&signed_tx).expect("failed to serialize NEAR transaction")
    }

    #[cfg(feature = "serde_json")]
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

#[cfg(all(test, feature = "borsh", feature = "serde", feature = "serde_json"))]
mod tests {
    use std::str::FromStr;
    use std::sync::Arc;

    use super::*;
    use crate::near::types::{
        AccessKey as OmniAccessKey, AccessKeyPermission as OmniAccessKeyPermission,
        Action as OmniAction, AddKeyAction as OmniAddKeyAction,
        CreateAccountAction as OmniCreateAccountAction, DelegateAction as OmniDelegateAction,
        DeleteAccountAction as OmniDeleteAccountAction, DeleteKeyAction as OmniDeleteKeyAction,
        DeployContractAction as OmniDeployContractAction,
        DeployGlobalContractAction as OmniDeployGlobalContractAction, ED25519Signature,
        FunctionCallAction as OmniFunctionCallAction,
        GlobalContractDeployMode as OmniGlobalContractDeployMode,
        GlobalContractIdentifier as OmniGlobalContractIdentifier, NearGas, NearToken,
        Secp256K1Signature, Signature as OmniSignature,
        SignedDelegateAction as OmniSignedDelegateAction, StakeAction as OmniStakeAction,
        TransferAction as OmniTransferAction,
        UseGlobalContractAction as OmniUseGlobalContractAction,
    };
    use crate::near::utils::PublicKeyStrExt;
    use near_crypto::{ED25519PublicKey, PublicKey, Signature};
    use near_crypto::{InMemorySigner, KeyType, Signer};
    use near_primitives::action::delegate::{DelegateAction, SignedDelegateAction};
    use near_primitives::action::{
        CreateAccountAction, DeleteAccountAction, DeleteKeyAction, DeployContractAction,
        DeployGlobalContractAction, FunctionCallAction, GlobalContractDeployMode,
        GlobalContractIdentifier, StakeAction, UseGlobalContractAction,
    };
    use near_primitives::{
        account::{AccessKey, AccessKeyPermission},
        action::{Action, AddKeyAction, TransferAction},
        hash::CryptoHash,
        transaction::Transaction as NearPrimitiveTransaction,
        transaction::TransactionV0,
        types::Balance,
        types::Gas,
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
        let signature = {
            use crate::near::utils::SignatureStrExt;
            "ed25519:3s1dvZdQtcAjBksMHFrysqvF63wnyMHPA4owNQmCJZ2EBakZEKdtMsLqrHdKWQjJbSRN6kRknN2WdwSBLWGCokXj".to_signature().unwrap()
        };
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
                    gas: Gas::from_gas(100),
                    deposit: Balance::from_yoctonear(1),
                }))],
                omni_actions: vec![OmniAction::FunctionCall(Box::new(OmniFunctionCallAction {
                    method_name: "function1".to_string(),
                    args: vec![0x01, 0x02, 0x03],
                    gas: NearGas::from_gas(100),
                    deposit: NearToken::from_yoctonear(1),
                }))],
            },
            // Transfer
            TestCase {
                signer_id: "alice.near",
                signer_public_key: "ed25519:6E8sCci9badyRkXb3JoRpBj5p8C6Tw41ELDZoiihKEtp",
                nonce: 1,
                receiver_id: "bob.near",
                block_hash: "4reLvkAWfqk5fsqio1KLudk46cqRz9erQdaHkWZKMJDZ",
                near_primitive_actions: vec![Action::Transfer(TransferAction { deposit: Balance::from_yoctonear(1) })],
                omni_actions: vec![OmniAction::Transfer(OmniTransferAction {
                    deposit: NearToken::from_yoctonear(1),
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
                    stake: Balance::from_yoctonear(1),
                    public_key: PublicKey::ED25519(ED25519PublicKey(
                        "ed25519:6E8sCci9badyRkXb3JoRpBj5p8C6Tw41ELDZoiihKEtp"
                            .try_ed25519_into_bytes()
                            .expect("Public key should be 32 bytes"),
                    )),
                }))],
                omni_actions: vec![OmniAction::Stake(Box::new(OmniStakeAction {
                    stake: NearToken::from_yoctonear(1),
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
            // Delegate
            TestCase {
                signer_id: "alice.near",
                signer_public_key: "ed25519:6E8sCci9badyRkXb3JoRpBj5p8C6Tw41ELDZoiihKEtp",
                nonce: 1,
                receiver_id: "bob.near",
                block_hash: "4reLvkAWfqk5fsqio1KLudk46cqRz9erQdaHkWZKMJDZ",
                near_primitive_actions: vec![Action::Delegate(Box::new(SignedDelegateAction{
                    delegate_action: DelegateAction {
                        sender_id: "alice.near".parse().unwrap(),
                        receiver_id: "bob.near".parse().unwrap(),
                        actions: vec![Action::CreateAccount(CreateAccountAction {}).try_into().unwrap()],
                        nonce: 1,
                        max_block_height: 1,
                        public_key: PublicKey::from_str("ed25519:6E8sCci9badyRkXb3JoRpBj5p8C6Tw41ELDZoiihKEtp").unwrap(),
                    },
                    signature: Signature::from_str("ed25519:3s1dvZdQtcAjBksMHFrysqvF63wnyMHPA4owNQmCJZ2EBakZEKdtMsLqrHdKWQjJbSRN6kRknN2WdwSBLWGCokXj").unwrap()
                }))],
                omni_actions: vec![OmniAction::Delegate(Box::new(OmniSignedDelegateAction {
                    delegate_action: OmniDelegateAction {
                        sender_id: "alice.near".parse().unwrap(),
                        receiver_id: "bob.near".parse().unwrap(),
                        actions: vec![OmniAction::CreateAccount(OmniCreateAccountAction {}).try_into().unwrap()],
                        nonce: U64(1),
                        max_block_height: U64(1),
                        public_key: "ed25519:6E8sCci9badyRkXb3JoRpBj5p8C6Tw41ELDZoiihKEtp".to_public_key().unwrap(),
                    },
                    signature
                }))],
            },
            // Use Global Contract
            TestCase {
                signer_id: "alice.near",
                signer_public_key: "ed25519:6E8sCci9badyRkXb3JoRpBj5p8C6Tw41ELDZoiihKEtp",
                nonce: 1,
                receiver_id: "bob.near",
                block_hash: "4reLvkAWfqk5fsqio1KLudk46cqRz9erQdaHkWZKMJDZ",
                near_primitive_actions: vec![Action::UseGlobalContract(Box::new(UseGlobalContractAction {
                    contract_identifier: GlobalContractIdentifier::CodeHash(CryptoHash::from_str("4reLvkAWfqk5fsqio1KLudk46cqRz9erQdaHkWZKMJDZ").unwrap()),
                }))],
                omni_actions: vec![
                    OmniAction::UseGlobalContract(Box::new(OmniUseGlobalContractAction {
                        contract_identifier: OmniGlobalContractIdentifier::CodeHash(BlockHash("4reLvkAWfqk5fsqio1KLudk46cqRz9erQdaHkWZKMJDZ".to_fixed_32_bytes().unwrap())),
                    })),
                ],
            },
            // Deploy Global Contract
            TestCase {
                signer_id: "alice.near",
                signer_public_key: "ed25519:6E8sCci9badyRkXb3JoRpBj5p8C6Tw41ELDZoiihKEtp",
                nonce: 1,
                receiver_id: "bob.near",
                block_hash: "4reLvkAWfqk5fsqio1KLudk46cqRz9erQdaHkWZKMJDZ",
                near_primitive_actions: vec![Action::DeployGlobalContract(DeployGlobalContractAction {
                    code: Arc::new([0x01, 0x02, 0x03]),
                    deploy_mode: GlobalContractDeployMode::AccountId,
                })],
                omni_actions: vec![
                    OmniAction::DeployGlobalContract(OmniDeployGlobalContractAction {
                        code: vec![0x01, 0x02, 0x03],
                        deploy_mode: OmniGlobalContractDeployMode::AccountId,
                    }),
                ],
            },
            // Transfer and Add Key
            TestCase {
                signer_id: "forgetful-parent.testnet",
                signer_public_key: "ed25519:6E8sCci9badyRkXb3JoRpBj5p8C6Tw41ELDZoiihKEtp",
                nonce: 1,
                receiver_id: "forgetful-parent.testnet",
                block_hash: "4reLvkAWfqk5fsqio1KLudk46cqRz9erQdaHkWZKMJDZ",
                near_primitive_actions: vec![
                    Action::Transfer(TransferAction { deposit: Balance::from_yoctonear(1) }),
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
                    OmniAction::Transfer(OmniTransferAction { deposit: NearToken::from_yoctonear(1) }),
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

            let omni_tx = NearTransaction::V0(super::TransactionV0 {
                signer_id: test_case.signer_id.parse().unwrap(),
                signer_public_key: test_case.signer_public_key.to_public_key().unwrap(),
                nonce: U64(test_case.nonce),
                receiver_id: test_case.receiver_id.parse().unwrap(),
                block_hash: test_case.block_hash.to_block_hash().unwrap(),
                actions: test_case.omni_actions.clone(),
            });

            let serialized_omni_tx = omni_tx.build_for_signing();

            assert_eq!(
                serialized_near_primitive_v0_tx, serialized_omni_tx,
                "Test case {i} failed: serialized transactions do not match.\nNEAR: {serialized_near_primitive_v0_tx:?}\nOmni: {serialized_omni_tx:?}"
            );
        }
    }

    #[test]
    fn test_compare_serde_json_with_near_primitives() {
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

            let serialized_near_primitive_v0_tx = serde_json::to_string(&near_primitive_v0_tx)
                .expect("failed to serialize NEAR transaction");

            let omni_tx = NearTransaction::V0(super::TransactionV0 {
                signer_id: test_case.signer_id.parse().unwrap(),
                signer_public_key: test_case.signer_public_key.to_public_key().unwrap(),
                nonce: U64(test_case.nonce),
                receiver_id: test_case.receiver_id.parse().unwrap(),
                block_hash: test_case.block_hash.to_block_hash().unwrap(),
                actions: test_case.omni_actions.clone(),
            });

            let serialized_omni_tx =
                serde_json::to_string(&omni_tx).expect("failed to serialize Omni transaction");

            // omni Action (JSON) ~= near-primitives Action (JSON)
            //
            // NOTE: nearcore serializes Gas values as JSON numbers for historical reasons, but it
            // can handle deserialization from both number and string values. NearGas, on the other
            // hand, serializes the values as JSON string and also can handle both number and
            // string values on deserialization
            assert_eq!(
                serialized_near_primitive_v0_tx.replace("\"gas\":100", "\"gas\":\"100\""), serialized_omni_tx,
                "Test case {i} failed: serialized transactions do not match.\nNEAR: {serialized_near_primitive_v0_tx:?}\nOmni: {serialized_omni_tx:?}"
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

            let omni_tx = NearTransaction::V0(super::TransactionV0 {
                signer_id: test_case.signer_id.parse().unwrap(),
                signer_public_key: test_case.signer_public_key.to_public_key().unwrap(),
                nonce: U64(test_case.nonce),
                receiver_id: test_case.receiver_id.parse().unwrap(),
                block_hash: test_case.block_hash.to_block_hash().unwrap(),
                actions: test_case.omni_actions.clone(),
            });

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
                "Test case {i} failed: serialized transactions do not match.\nNEAR: {serialized_omni_tx:?}\nOmni: {encoded_signed_tx:?}"
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

            let omni_tx = NearTransaction::V0(super::TransactionV0 {
                signer_id: test_case.signer_id.parse().unwrap(),
                signer_public_key: test_case.signer_public_key.to_public_key().unwrap(),
                nonce: U64(test_case.nonce),
                receiver_id: test_case.receiver_id.parse().unwrap(),
                block_hash: test_case.block_hash.to_block_hash().unwrap(),
                actions: test_case.omni_actions.clone(),
            });

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
                "Test case {i} failed: serialized transactions do not match.\nNEAR: {serialized_omni_tx:?}\nOmni: {encoded_signed_tx:?}"
            );
        }
    }

    #[test]
    fn test_build_for_signing_for_omni_against_near_primitives_using_json_input() {
        let input = r#"
        {
            "signer_id": "86a315fdc1c4211787aa2fd78a50041ee581c7fff6cec2535ebec14af5c40381",
            "public_key": "ed25519:A4ZsCYMqJ1oHFGR2g2mFrwhQvaWmyz8K5c5FvfxEPF52",
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
            "public_key": "ed25519:6E8sCci9badyRkXb3JoRpBj5p8C6Tw41ELDZoiihKEtp",
            "nonce": 1,
            "receiver_id": "forgetful-parent.testnet",
            "block_hash": "4reLvkAWfqk5fsqio1KLudk46cqRz9erQdaHkWZKMJDZ",
            "actions": [
                { "Transfer": { "deposit": "1" } }
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

    #[test]
    fn test_transaction_v1_with_priority_fee() {
        use near_primitives::transaction::Transaction as NearPrimitiveTransaction;
        use near_primitives::transaction::TransactionV1;

        let signer_id = "alice.near";
        let signer_public_key = "ed25519:6E8sCci9badyRkXb3JoRpBj5p8C6Tw41ELDZoiihKEtp";
        let nonce = 1;
        let receiver_id = "bob.near";
        let block_hash_str = "4reLvkAWfqk5fsqio1KLudk46cqRz9erQdaHkWZKMJDZ";
        let priority_fee = 1000u64;

        // Create a NearPrimitive V1 transaction
        let near_primitive_v1_tx = TransactionV1 {
            signer_id: signer_id.parse().unwrap(),
            public_key: PublicKey::ED25519(ED25519PublicKey(
                signer_public_key
                    .to_public_key_as_bytes()
                    .unwrap()
                    .try_into()
                    .expect("Public key should be 32 bytes"),
            )),
            nonce,
            receiver_id: receiver_id.parse().unwrap(),
            block_hash: CryptoHash(block_hash_str.to_fixed_32_bytes().unwrap()),
            actions: vec![Action::Transfer(TransferAction {
                deposit: Balance::from_yoctonear(1),
            })],
            priority_fee,
        };

        let near_primitive_tx = NearPrimitiveTransaction::V1(near_primitive_v1_tx);
        let serialized_near_primitive_v1_tx =
            borsh::to_vec(&near_primitive_tx).expect("failed to serialize NEAR transaction");

        // Create our Omni V1 transaction
        let omni_tx = NearTransaction::V1(super::TransactionV1 {
            signer_id: signer_id.parse().unwrap(),
            signer_public_key: signer_public_key.to_public_key().unwrap(),
            nonce: U64(nonce),
            receiver_id: receiver_id.parse().unwrap(),
            block_hash: block_hash_str.to_block_hash().unwrap(),
            actions: vec![OmniAction::Transfer(OmniTransferAction {
                deposit: U128(1),
            })],
            priority_fee,
        });

        let serialized_omni_tx = omni_tx.build_for_signing();

        assert_eq!(
            serialized_near_primitive_v1_tx, serialized_omni_tx,
            "V1 transactions do not match.\nNEAR: {serialized_near_primitive_v1_tx:?}\nOmni: {serialized_omni_tx:?}"
        );

        // Verify priority_fee accessor works
        assert_eq!(omni_tx.priority_fee(), Some(priority_fee));
    }

    #[test]
    fn test_transaction_v0_backwards_compatibility() {
        // Test that V0 transactions serialize without a version byte
        let signer_id = "alice.near";
        let signer_public_key = "ed25519:6E8sCci9badyRkXb3JoRpBj5p8C6Tw41ELDZoiihKEtp";
        let nonce = 1;
        let receiver_id = "bob.near";
        let block_hash_str = "4reLvkAWfqk5fsqio1KLudk46cqRz9erQdaHkWZKMJDZ";

        let omni_tx = NearTransaction::V0(super::TransactionV0 {
            signer_id: signer_id.parse().unwrap(),
            signer_public_key: signer_public_key.to_public_key().unwrap(),
            nonce: U64(nonce),
            receiver_id: receiver_id.parse().unwrap(),
            block_hash: block_hash_str.to_block_hash().unwrap(),
            actions: vec![OmniAction::Transfer(OmniTransferAction {
                deposit: U128(1),
            })],
        });

        let serialized = omni_tx.build_for_signing();

        // First byte should NOT be 1 (the version marker for V1)
        // It should be the length of the signer_id in little-endian
        let signer_id_len = signer_id.len() as u8;
        assert_eq!(serialized[0], signer_id_len);
        assert_eq!(serialized[1], 0); // Second byte should be 0 for V0

        // Verify priority_fee accessor returns None for V0
        assert_eq!(omni_tx.priority_fee(), None);
    }

    #[test]
    fn test_transaction_v1_serialization_starts_with_version() {
        // Test that V1 transactions serialize with a version byte of 1
        let signer_id = "alice.near";
        let signer_public_key = "ed25519:6E8sCci9badyRkXb3JoRpBj5p8C6Tw41ELDZoiihKEtp";
        let nonce = 1;
        let receiver_id = "bob.near";
        let block_hash_str = "4reLvkAWfqk5fsqio1KLudk46cqRz9erQdaHkWZKMJDZ";
        let priority_fee = 1000u64;

        let omni_tx = NearTransaction::V1(super::TransactionV1 {
            signer_id: signer_id.parse().unwrap(),
            signer_public_key: signer_public_key.to_public_key().unwrap(),
            nonce: U64(nonce),
            receiver_id: receiver_id.parse().unwrap(),
            block_hash: block_hash_str.to_block_hash().unwrap(),
            actions: vec![OmniAction::Transfer(OmniTransferAction {
                deposit: U128(1),
            })],
            priority_fee,
        });

        let serialized = omni_tx.build_for_signing();

        // First byte should be 1 (the version marker for V1)
        assert_eq!(serialized[0], 1);
        // Second byte should be the length of the signer_id (not 0)
        let signer_id_len = signer_id.len() as u8;
        assert_eq!(serialized[1], signer_id_len);
    }

    #[test]
    fn test_deserialize_v0_transaction() {
        // Create a V0 transaction, serialize it, and deserialize it back
        let signer_id = "alice.near";
        let signer_public_key = "ed25519:6E8sCci9badyRkXb3JoRpBj5p8C6Tw41ELDZoiihKEtp";
        let nonce = 1;
        let receiver_id = "bob.near";
        let block_hash_str = "4reLvkAWfqk5fsqio1KLudk46cqRz9erQdaHkWZKMJDZ";

        let original_tx = NearTransaction::V0(super::TransactionV0 {
            signer_id: signer_id.parse().unwrap(),
            signer_public_key: signer_public_key.to_public_key().unwrap(),
            nonce: U64(nonce),
            receiver_id: receiver_id.parse().unwrap(),
            block_hash: block_hash_str.to_block_hash().unwrap(),
            actions: vec![OmniAction::Transfer(OmniTransferAction {
                deposit: U128(1),
            })],
        });

        let serialized = borsh::to_vec(&original_tx).expect("Failed to serialize");
        let deserialized: NearTransaction =
            borsh::from_slice(&serialized).expect("Failed to deserialize");

        // Verify it's a V0 transaction
        assert!(matches!(deserialized, NearTransaction::V0(_)));
        assert_eq!(deserialized.signer_id().as_str(), signer_id);
        assert_eq!(deserialized.receiver_id().as_str(), receiver_id);
        assert_eq!(deserialized.nonce(), U64(nonce));
        assert_eq!(deserialized.priority_fee(), None);
    }

    #[test]
    fn test_deserialize_v1_transaction() {
        // Create a V1 transaction, serialize it, and deserialize it back
        let signer_id = "alice.near";
        let signer_public_key = "ed25519:6E8sCci9badyRkXb3JoRpBj5p8C6Tw41ELDZoiihKEtp";
        let nonce = 1;
        let receiver_id = "bob.near";
        let block_hash_str = "4reLvkAWfqk5fsqio1KLudk46cqRz9erQdaHkWZKMJDZ";
        let priority_fee = 1000u64;

        let original_tx = NearTransaction::V1(super::TransactionV1 {
            signer_id: signer_id.parse().unwrap(),
            signer_public_key: signer_public_key.to_public_key().unwrap(),
            nonce: U64(nonce),
            receiver_id: receiver_id.parse().unwrap(),
            block_hash: block_hash_str.to_block_hash().unwrap(),
            actions: vec![OmniAction::Transfer(OmniTransferAction {
                deposit: U128(1),
            })],
            priority_fee,
        });

        let serialized = borsh::to_vec(&original_tx).expect("Failed to serialize");
        let deserialized: NearTransaction =
            borsh::from_slice(&serialized).expect("Failed to deserialize");

        // Verify it's a V1 transaction
        assert!(matches!(deserialized, NearTransaction::V1(_)));
        assert_eq!(deserialized.signer_id().as_str(), signer_id);
        assert_eq!(deserialized.receiver_id().as_str(), receiver_id);
        assert_eq!(deserialized.nonce(), U64(nonce));
        assert_eq!(deserialized.priority_fee(), Some(priority_fee));
    }
}
