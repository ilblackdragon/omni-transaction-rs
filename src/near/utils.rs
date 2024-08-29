use bs58;
use std::convert::TryInto;

use super::types::{ED25519PublicKey, PublicKey, Secp256K1PublicKey};

/// Trait to extend `&str` with methods for parsing public keys and block hashes.
pub trait PublicKeyStrExt {
    /// Converts a string in base58 (with prefixes like "ed25519:" or "secp256k1:") into a `PublicKey`.
    fn to_public_key(&self) -> Result<PublicKey, String>;

    fn to_public_key_as_bytes(&self) -> Result<[u8; 32], String>;

    // /// Converts a string in base58 into a block hash `[u8; 32]`.
    // fn to_block_hash(&self) -> Result<[u8; 32], String>;

    /// Converts a string in base58 into a 32 byte vector.
    fn to_fixed_32_bytes(&self) -> Result<[u8; 32], String>;

    /// Converts a string in base58 into a 64 byte vector.
    fn to_fixed_64_bytes(&self) -> Result<[u8; 64], String>;
}

impl PublicKeyStrExt for str {
    fn to_fixed_64_bytes(&self) -> Result<[u8; 64], String> {
        let bytes = bs58::decode(self).into_vec().map_err(|e| e.to_string())?;
        bytes
            .try_into()
            .map_err(|_| "Block hash should be exactly 64 bytes long".to_string())
    }

    fn to_fixed_32_bytes(&self) -> Result<[u8; 32], String> {
        let bytes = bs58::decode(self).into_vec().map_err(|e| e.to_string())?;
        bytes
            .try_into()
            .map_err(|_| "Block hash should be exactly 32 bytes long".to_string())
    }

    fn to_public_key(&self) -> Result<PublicKey, String> {
        if let Some(rest) = self.strip_prefix("ed25519:") {
            let bytes = bs58::decode(rest).into_vec().map_err(|e| e.to_string())?;
            Ok(PublicKey::ED25519(ED25519PublicKey(vec_to_fixed(bytes))))
        } else if let Some(rest) = self.strip_prefix("secp256k1:") {
            let bytes = bs58::decode(rest).into_vec().map_err(|e| e.to_string())?;
            Ok(PublicKey::SECP256K1(Secp256K1PublicKey(vec_to_fixed(
                bytes,
            ))))
        } else {
            Err("Unknown key type or invalid format".into())
        }
    }

    fn to_public_key_as_bytes(&self) -> Result<[u8; 32], String> {
        if let Some(rest) = self.strip_prefix("ed25519:") {
            let bytes = bs58::decode(rest).into_vec().map_err(|e| e.to_string())?;
            Ok(vec_to_fixed(bytes))
        } else if let Some(rest) = self.strip_prefix("secp256k1:") {
            let bytes = bs58::decode(rest).into_vec().map_err(|e| e.to_string())?;
            Ok(vec_to_fixed(bytes))
        } else {
            Err("Unknown key type or invalid format".into())
        }
    }

    // fn to_block_hash(&self) -> Result<[u8; 32], String> {
    //     let bytes = bs58::decode(self).into_vec().map_err(|e| e.to_string())?;
    //     bytes
    //         .try_into()
    //         .map_err(|_| "Block hash should be exactly 32 bytes long".to_string())
    // }
}

fn vec_to_fixed<T, const N: usize>(v: Vec<T>) -> [T; N] {
    v.try_into()
        .unwrap_or_else(|v: Vec<T>| panic!("Expected a Vec of length {} but it was {}", N, v.len()))
}
