use crate::constants::{ED25519_PUBLIC_KEY_LENGTH, SECP256K1_PUBLIC_KEY_LENGTH};

use std::io::{Error, Write};

use borsh::BorshSerialize;

// Actions
#[derive(Debug, Clone, BorshSerialize)]
pub enum Action {
    CreateAccount(CreateAccountAction),
    DeployContract(DeployContractAction),
    FunctionCall(Box<FunctionCallAction>),
    Transfer(TransferAction),
}

#[derive(Debug, Clone, BorshSerialize)]
pub struct CreateAccountAction {}

#[derive(Debug, Clone, BorshSerialize)]
pub struct TransferAction {
    pub deposit: u128,
}

#[derive(Debug, Clone, BorshSerialize)]
pub struct FunctionCallAction {
    pub method_name: String,
    pub args: Vec<u8>,
    pub gas: u64,
    pub deposit: u128,
}

#[derive(Debug, Clone, BorshSerialize)]
pub struct DeployContractAction {
    pub code: Vec<u8>,
}

#[derive(Debug, Clone, Eq, Ord, PartialEq, PartialOrd)]
pub struct Secp256K1PublicKey([u8; SECP256K1_PUBLIC_KEY_LENGTH]);

#[derive(Debug, Clone, Eq, Ord, PartialEq, PartialOrd)]
pub struct ED25519PublicKey(pub [u8; ED25519_PUBLIC_KEY_LENGTH]);

#[derive(Debug, Clone, PartialEq, PartialOrd, Ord, Eq)]
pub enum PublicKey {
    /// 256 bit elliptic curve based public-key.
    ED25519(ED25519PublicKey),
    /// 512 bit elliptic curve based public-key used in Bitcoin's public-key cryptography.
    SECP256K1(Secp256K1PublicKey),
}

impl BorshSerialize for PublicKey {
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<(), Error> {
        match self {
            PublicKey::ED25519(public_key) => {
                BorshSerialize::serialize(&0u8, writer)?;
                writer.write_all(&public_key.0)?;
            }
            PublicKey::SECP256K1(public_key) => {
                BorshSerialize::serialize(&1u8, writer)?;
                writer.write_all(&public_key.0)?;
            }
        }
        Ok(())
    }
}

impl From<[u8; 64]> for Secp256K1PublicKey {
    fn from(data: [u8; 64]) -> Self {
        Secp256K1PublicKey(data)
    }
}
