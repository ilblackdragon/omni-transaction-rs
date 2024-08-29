use crate::constants::{ED25519_PUBLIC_KEY_LENGTH, SECP256K1_PUBLIC_KEY_LENGTH};
use near_sdk::{
    serde::{Deserialize, Serialize},
    AccountId,
};
use serde_big_array::BigArray;

use std::io::{Error, Write};

use borsh::BorshSerialize;

// Actions
#[derive(Serialize, Deserialize, Debug, Clone, BorshSerialize)]
#[serde(crate = "near_sdk::serde")]
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
}

#[derive(Serialize, Deserialize, Debug, Clone, BorshSerialize)]
#[serde(crate = "near_sdk::serde")]
pub struct CreateAccountAction {}

#[derive(Serialize, Deserialize, Debug, Clone, BorshSerialize)]
#[serde(crate = "near_sdk::serde")]
pub struct DeployContractAction {
    pub code: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug, Clone, BorshSerialize)]
#[serde(crate = "near_sdk::serde")]
pub struct FunctionCallAction {
    pub method_name: String,
    pub args: Vec<u8>,
    pub gas: u64,
    pub deposit: u128,
}

#[derive(Serialize, Deserialize, Debug, Clone, BorshSerialize)]
#[serde(crate = "near_sdk::serde")]
pub struct TransferAction {
    pub deposit: u128,
}

#[derive(Serialize, Deserialize, Debug, Clone, BorshSerialize)]
#[serde(crate = "near_sdk::serde")]
pub struct StakeAction {
    /// Amount of tokens to stake.
    pub stake: u128,
    /// Validator key which will be used to sign transactions on behalf of signer_id
    pub public_key: PublicKey,
}

#[derive(Serialize, Deserialize, Debug, Clone, BorshSerialize)]
#[serde(crate = "near_sdk::serde")]
pub struct AddKeyAction {
    /// A public key which will be associated with an access_key
    pub public_key: PublicKey,
    /// An access key with the permission
    pub access_key: AccessKey,
}

#[derive(Serialize, Deserialize, Debug, Clone, BorshSerialize)]
#[serde(crate = "near_sdk::serde")]
pub struct AccessKey {
    /// Nonce for this access key, used for tx nonce generation. When access key is created, nonce
    /// is set to `(block_height - 1) * 1e6` to avoid tx hash collision on access key re-creation.
    /// See <https://github.com/near/nearcore/issues/3779> for more details.
    pub nonce: u64,

    /// Defines permissions for this access key.
    pub permission: AccessKeyPermission,
}

#[derive(Serialize, Deserialize, Debug, Clone, BorshSerialize)]
#[serde(crate = "near_sdk::serde")]
pub enum AccessKeyPermission {
    FunctionCall(FunctionCallPermission),

    /// Grants full access to the account.
    /// NOTE: It's used to replace account-level public keys.
    FullAccess,
}

#[derive(Serialize, Deserialize, Debug, Clone, BorshSerialize)]
#[serde(crate = "near_sdk::serde")]
pub struct FunctionCallPermission {
    pub allowance: Option<u128>,
    pub receiver_id: String,
    pub method_names: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, BorshSerialize)]
#[serde(crate = "near_sdk::serde")]
pub struct DeleteKeyAction {
    /// A public key associated with the access_key to be deleted.
    pub public_key: PublicKey,
}

#[derive(Serialize, Deserialize, Debug, Clone, BorshSerialize)]
#[serde(crate = "near_sdk::serde")]
pub struct DeleteAccountAction {
    pub beneficiary_id: AccountId,
}
// Public Key

#[derive(Serialize, Deserialize, Debug, Clone, Eq, Ord, PartialEq, PartialOrd)]
#[serde(crate = "near_sdk::serde")]
pub struct Secp256K1PublicKey(#[serde(with = "BigArray")] pub [u8; SECP256K1_PUBLIC_KEY_LENGTH]);

#[derive(Serialize, Deserialize, Debug, Clone, Eq, Ord, PartialEq, PartialOrd)]
#[serde(crate = "near_sdk::serde")]
pub struct ED25519PublicKey(pub [u8; ED25519_PUBLIC_KEY_LENGTH]);

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, PartialOrd, Ord, Eq)]
#[serde(crate = "near_sdk::serde")]
pub enum PublicKey {
    /// 256 bit elliptic curve based public-key.
    ED25519(ED25519PublicKey),
    /// 512 bit elliptic curve based public-key used in Bitcoin's public-key cryptography.
    SECP256K1(Secp256K1PublicKey),
}

impl BorshSerialize for PublicKey {
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<(), Error> {
        match self {
            Self::ED25519(public_key) => {
                BorshSerialize::serialize(&0u8, writer)?;
                writer.write_all(&public_key.0)?;
            }
            Self::SECP256K1(public_key) => {
                BorshSerialize::serialize(&1u8, writer)?;
                writer.write_all(&public_key.0)?;
            }
        }
        Ok(())
    }
}

impl From<[u8; 64]> for Secp256K1PublicKey {
    fn from(data: [u8; 64]) -> Self {
        Self(data)
    }
}

impl From<[u8; 32]> for ED25519PublicKey {
    fn from(data: [u8; 32]) -> Self {
        Self(data)
    }
}
