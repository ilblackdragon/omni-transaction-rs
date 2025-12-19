use crate::near::types::{BlockHash, PublicKey, Signature};
#[cfg(feature = "serde")]
use crate::near::utils::base64_serialization;
#[cfg(feature = "borsh")]
use borsh::{BorshDeserialize, BorshSerialize};
use near_account_id::AccountId;
#[cfg(feature = "schemars")]
use schemars::JsonSchema;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use super::{U128, U64};

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "borsh", derive(BorshSerialize, BorshDeserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub enum Action {
    /// Create an (sub)account using a transaction `receiver_id` as an ID for
    /// a new account ID must pass validation rules described here
    /// <http://nomicon.io/Primitives/Account.html>.
    CreateAccount(CreateAccountAction),
    /// Sets a Wasm code to a receiver_id
    DeployContract(DeployContractAction),
    FunctionCall(Box<FunctionCallAction>),
    Transfer(TransferAction),
    Stake(Box<StakeAction>),
    AddKey(Box<AddKeyAction>),
    DeleteKey(Box<DeleteKeyAction>),
    DeleteAccount(DeleteAccountAction),
    Delegate(Box<SignedDelegateAction>),
    DeployGlobalContract(DeployGlobalContractAction),
    UseGlobalContract(Box<UseGlobalContractAction>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "borsh", derive(BorshSerialize, BorshDeserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct DeployGlobalContractAction {
    #[cfg_attr(feature = "serde", serde(with = "base64_serialization"))]
    #[cfg_attr(feature = "schemars", schemars(with = "String", extend("contentMediaType"="application/octet-stream", "contentEncoding" = "base64", "format" = "byte")))]
    pub code: Vec<u8>,
    pub deploy_mode: GlobalContractDeployMode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "borsh", derive(BorshSerialize, BorshDeserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct UseGlobalContractAction {
    pub contract_identifier: GlobalContractIdentifier,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "borsh", derive(BorshSerialize, BorshDeserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub enum GlobalContractDeployMode {
    /// Contract is deployed under its code hash.
    /// Users will be able reference it by that hash.
    /// This effectively makes the contract immutable.
    CodeHash,
    /// Contract is deployed under the owner account id.
    /// Users will be able reference it by that account id.
    /// This allows the owner to update the contract for all its users.
    AccountId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "borsh", derive(BorshSerialize, BorshDeserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub enum GlobalContractIdentifier {
    CodeHash(BlockHash),
    AccountId(AccountId),
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "borsh", derive(BorshSerialize, BorshDeserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct CreateAccountAction {}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "borsh", derive(BorshSerialize, BorshDeserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct DeployContractAction {
    #[cfg_attr(feature = "serde", serde(with = "base64_serialization"))]
    #[cfg_attr(feature = "schemars", schemars(with = "String", extend("contentMediaType"="application/octet-stream", "contentEncoding" = "base64", "format" = "byte")))]
    pub code: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "borsh", derive(BorshSerialize, BorshDeserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct FunctionCallAction {
    pub method_name: String,
    #[cfg_attr(feature = "serde", serde(with = "base64_serialization"))]
    #[cfg_attr(feature = "schemars", schemars(with = "String", extend("contentMediaType"="application/octet-stream", "contentEncoding" = "base64", "format" = "byte")))]
    pub args: Vec<u8>,
    pub gas: U64,
    pub deposit: U128,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "borsh", derive(BorshSerialize, BorshDeserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct TransferAction {
    pub deposit: U128,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "borsh", derive(BorshSerialize, BorshDeserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct StakeAction {
    /// Amount of tokens to stake.
    pub stake: U128,
    /// Validator key which will be used to sign transactions on behalf of signer_id
    pub public_key: PublicKey,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "borsh", derive(BorshSerialize, BorshDeserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct AddKeyAction {
    /// A public key which will be associated with an access_key
    pub public_key: PublicKey,
    /// An access key with the permission
    pub access_key: AccessKey,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "borsh", derive(BorshSerialize, BorshDeserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct AccessKey {
    /// Nonce for this access key, used for tx nonce generation. When access key is created, nonce
    /// is set to `(block_height - 1) * 1e6` to avoid tx hash collision on access key re-creation.
    /// See <https://github.com/near/nearcore/issues/3779> for more details.
    pub nonce: U64,
    /// Defines permissions for this access key.
    pub permission: AccessKeyPermission,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "borsh", derive(BorshSerialize, BorshDeserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub enum AccessKeyPermission {
    FunctionCall(FunctionCallPermission),
    /// Grants full access to the account.
    /// NOTE: It's used to replace account-level public keys.
    FullAccess,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "borsh", derive(BorshSerialize, BorshDeserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct FunctionCallPermission {
    pub allowance: Option<U128>,
    pub receiver_id: String,
    pub method_names: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "borsh", derive(BorshSerialize, BorshDeserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct DeleteKeyAction {
    /// A public key associated with the access_key to be deleted.
    pub public_key: PublicKey,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "borsh", derive(BorshSerialize, BorshDeserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct DeleteAccountAction {
    pub beneficiary_id: AccountId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "borsh", derive(BorshSerialize, BorshDeserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct NonDelegateAction(Action);

impl TryFrom<Action> for NonDelegateAction {
    type Error = ();
    fn try_from(action: Action) -> Result<Self, Self::Error> {
        if let Action::Delegate(_) = action {
            return Err(());
        }
        Ok(Self(action))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "borsh", derive(BorshSerialize, BorshDeserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct DelegateAction {
    pub sender_id: AccountId,
    pub receiver_id: AccountId,
    pub actions: Vec<NonDelegateAction>,
    pub nonce: U64,
    pub max_block_height: U64,
    pub public_key: PublicKey,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "borsh", derive(BorshSerialize, BorshDeserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct SignedDelegateAction {
    pub delegate_action: DelegateAction,
    pub signature: Signature,
}

#[cfg(all(test, feature = "borsh", feature = "serde", feature = "serde_json"))]
mod tests {
    use super::*;
    use crate::constants::ED25519_PUBLIC_KEY_LENGTH;
    use crate::near::types::public_key::ED25519PublicKey;
    use near_primitives::types::{Balance, Gas};
    use serde_json;

    use near_primitives::action::{
        Action as NearPrimitiveAction, AddKeyAction as NearPrimitiveAddKeyAction,
        DeleteAccountAction as NearPrimitiveDeleteAccountAction,
        DeleteKeyAction as NearPrimitiveDeleteKeyAction,
        DeployContractAction as NearPrimitiveDeployContractAction,
        DeployGlobalContractAction as NearPrimitiveDeployGlobalContractAction,
        FunctionCallAction as NearPrimitiveFunctionCallAction,
        GlobalContractDeployMode as NearPrimitiveGlobalContractDeployMode,
        GlobalContractIdentifier as NearPrimitiveGlobalContractIdentifier,
        StakeAction as NearPrimitiveStakeAction, TransferAction as NearPrimitiveTransferAction,
        UseGlobalContractAction as NearPrimitiveUseGlobalContractAction,
    };

    fn get_actions() -> Vec<(Action, NearPrimitiveAction)> {
        vec![
            (
                Action::CreateAccount(CreateAccountAction {}),
                NearPrimitiveAction::CreateAccount(near_primitives::action::CreateAccountAction {}),
            ),
            (
                Action::DeployContract(DeployContractAction {
                    code: vec![1, 2, 3],
                }),
                NearPrimitiveAction::DeployContract(NearPrimitiveDeployContractAction {
                    code: vec![1, 2, 3],
                }),
            ),
            (
                Action::FunctionCall(Box::new(FunctionCallAction {
                    method_name: "test".to_string(),
                    args: vec![4, 5, 6],
                    gas: U64(1000000),
                    deposit: U128(0),
                })),
                NearPrimitiveAction::FunctionCall(Box::new(NearPrimitiveFunctionCallAction {
                    method_name: "test".to_string(),
                    args: vec![4, 5, 6],
                    gas: Gas::from_gas(1000000),
                    deposit: Balance::ZERO,
                })),
            ),
            (
                Action::Transfer(TransferAction {
                    deposit: U128(1000000000),
                }),
                NearPrimitiveAction::Transfer(NearPrimitiveTransferAction {
                    deposit: Balance::from_yoctonear(1000000000),
                }),
            ),
            (
                Action::Stake(Box::new(StakeAction {
                    stake: U128(100000000),
                    public_key: PublicKey::ED25519(ED25519PublicKey(
                        [0; ED25519_PUBLIC_KEY_LENGTH],
                    )),
                })),
                NearPrimitiveAction::Stake(Box::new(NearPrimitiveStakeAction {
                    stake: Balance::from_yoctonear(100000000),
                    public_key: near_crypto::PublicKey::ED25519(near_crypto::ED25519PublicKey(
                        [0; ED25519_PUBLIC_KEY_LENGTH],
                    )),
                })),
            ),
            (
                Action::AddKey(Box::new(AddKeyAction {
                    public_key: PublicKey::ED25519(ED25519PublicKey(
                        [1; ED25519_PUBLIC_KEY_LENGTH],
                    )),
                    access_key: AccessKey {
                        nonce: U64(0),
                        permission: AccessKeyPermission::FullAccess,
                    },
                })),
                NearPrimitiveAction::AddKey(Box::new(NearPrimitiveAddKeyAction {
                    public_key: near_crypto::PublicKey::ED25519(near_crypto::ED25519PublicKey(
                        [1; ED25519_PUBLIC_KEY_LENGTH],
                    )),
                    access_key: near_primitives::account::AccessKey {
                        nonce: 0,
                        permission: near_primitives::account::AccessKeyPermission::FullAccess,
                    },
                })),
            ),
            (
                Action::DeleteKey(Box::new(DeleteKeyAction {
                    public_key: PublicKey::ED25519(ED25519PublicKey(
                        [2; ED25519_PUBLIC_KEY_LENGTH],
                    )),
                })),
                NearPrimitiveAction::DeleteKey(Box::new(NearPrimitiveDeleteKeyAction {
                    public_key: near_crypto::PublicKey::ED25519(near_crypto::ED25519PublicKey(
                        [2; ED25519_PUBLIC_KEY_LENGTH],
                    )),
                })),
            ),
            (
                Action::DeleteAccount(DeleteAccountAction {
                    beneficiary_id: "alice.near".parse().unwrap(),
                }),
                NearPrimitiveAction::DeleteAccount(NearPrimitiveDeleteAccountAction {
                    beneficiary_id: "alice.near".parse().unwrap(),
                }),
            ),
            (
                Action::DeployGlobalContract(DeployGlobalContractAction {
                    code: vec![3, 4, 5],
                    deploy_mode: GlobalContractDeployMode::CodeHash,
                }),
                NearPrimitiveAction::DeployGlobalContract(
                    NearPrimitiveDeployGlobalContractAction {
                        code: std::sync::Arc::new([3, 4, 5]),
                        deploy_mode: NearPrimitiveGlobalContractDeployMode::CodeHash,
                    },
                ),
            ),
            (
                Action::UseGlobalContract(Box::new(UseGlobalContractAction {
                    contract_identifier: GlobalContractIdentifier::CodeHash(BlockHash([4; 32])),
                })),
                NearPrimitiveAction::UseGlobalContract(Box::new(
                    NearPrimitiveUseGlobalContractAction {
                        contract_identifier: NearPrimitiveGlobalContractIdentifier::CodeHash(
                            near_primitives::hash::CryptoHash([4; 32]),
                        ),
                    },
                )),
            ),
        ]
    }

    #[test]
    fn test_action_serialization() {
        let action_pairs = get_actions();

        for (action, near_primitive_action) in action_pairs {
            let serialized =
                serde_json::to_string(&action).expect("Failed to serialize action to JSON");

            let deserialized: Action =
                serde_json::from_str(&serialized).expect("Failed to deserialize action from JSON");

            assert_eq!(
                    action, deserialized,
                    "Serialization/Deserialization mismatch: original action: {action:?}, deserialized action: {deserialized:?}"
                );

            let serialized_near_primitive = serde_json::to_string(&near_primitive_action)
                .expect("Failed to serialize action to JSON");

            assert_eq!(serialized, serialized_near_primitive);
        }
    }

    #[test]
    fn test_action_borsh_serialization() {
        let action_pairs = get_actions();

        for (action, near_primitive_action) in action_pairs {
            let serialized = borsh::to_vec(&action).expect("Failed to serialize action to borsh");

            let deserialized: Action = Action::try_from_slice(&serialized)
                .expect("Failed to deserialize action from borsh");

            assert_eq!(
                action, deserialized,
                "Serialization/Deserialization mismatch: original action: {action:?}, deserialized action: {deserialized:?}"
            );

            let serialized_near_primitive =
                borsh::to_vec(&near_primitive_action).expect("Failed to serialize action to borsh");

            assert_eq!(serialized, serialized_near_primitive);
        }
    }
}
