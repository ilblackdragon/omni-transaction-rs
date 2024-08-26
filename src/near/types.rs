use crate::constants::{ED25519_PUBLIC_KEY_LENGTH, SECP256K1_PUBLIC_KEY_LENGTH};
use near_sdk::serde::{Deserialize, Serialize};
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
