use bs58;
use std::convert::TryInto;

use crate::constants::{ED25519_PUBLIC_KEY_LENGTH, SECP256K1_PUBLIC_KEY_LENGTH};

use super::types::{ED25519PublicKey, PublicKey, Secp256K1PublicKey};

/// Trait to extend `&str` with methods for parsing public keys and block hashes.
pub trait PublicKeyStrExt {
    /// Converts a string in base58 (with prefixes like "ed25519:" or "secp256k1:") into a `PublicKey`.
    fn to_public_key(&self) -> Result<PublicKey, String>;

    /// Converts a string in base58 (with prefixes like "ed25519:" or "secp256k1:") into a byte vector.
    fn to_public_key_as_bytes(&self) -> Result<Vec<u8>, String>;

    /// Converts a string in base58 into a 32-byte array.
    fn to_fixed_32_bytes(&self) -> Result<[u8; 32], String>;

    /// Converts a string in base58 into a 64-byte array.
    fn to_fixed_64_bytes(&self) -> Result<[u8; 64], String>;

    /// Converts a string in base58 with a "ed25519:" prefix into a 64-byte array.
    fn try_ed25519_into_bytes(&self) -> Result<[u8; ED25519_PUBLIC_KEY_LENGTH], String>;

    /// Converts a string in base58 with a "secp256k1:" prefix into a 64-byte array.
    fn try_secp256k1_into_bytes(&self) -> Result<[u8; SECP256K1_PUBLIC_KEY_LENGTH], String>;
}

impl PublicKeyStrExt for str {
    fn to_fixed_64_bytes(&self) -> Result<[u8; 64], String> {
        decode_base58_to_fixed_bytes::<64>(self)
    }

    fn to_fixed_32_bytes(&self) -> Result<[u8; 32], String> {
        decode_base58_to_fixed_bytes::<32>(self)
    }

    fn to_public_key(&self) -> Result<PublicKey, String> {
        let bytes = self.to_public_key_as_bytes()?;
        if self.starts_with("ed25519:") {
            Ok(PublicKey::ED25519(ED25519PublicKey(
                bytes
                    .try_into()
                    .map_err(|_| "Invalid length for ED25519 key".to_string())?,
            )))
        } else if self.starts_with("secp256k1:") {
            Ok(PublicKey::SECP256K1(Secp256K1PublicKey(
                bytes
                    .try_into()
                    .map_err(|_| "Invalid length for SECP256K1 key".to_string())?,
            )))
        } else {
            Err("Unknown key type".into())
        }
    }

    fn try_ed25519_into_bytes(&self) -> Result<[u8; 32], String> {
        self.strip_prefix("ed25519:")
            .ok_or_else(|| "Invalid ED25519 key format".to_string())
            .and_then(|rest| {
                let bytes = bs58::decode(rest)
                    .into_vec()
                    .map_err(|e| format!("Failed to decode base58: {}", e))?;

                bytes
                    .try_into()
                    .map_err(|_| "Public key should be 32 bytes".to_string())
            })
    }

    fn try_secp256k1_into_bytes(&self) -> Result<[u8; 64], String> {
        self.strip_prefix("secp256k1:")
            .ok_or_else(|| "Invalid SECP256K1 key format".to_string())
            .and_then(|rest| {
                let bytes = bs58::decode(rest)
                    .into_vec()
                    .map_err(|e| format!("Failed to decode base58: {}", e))?;

                bytes
                    .try_into()
                    .map_err(|_| "Public key should be 64 bytes".to_string())
            })
    }

    fn to_public_key_as_bytes(&self) -> Result<Vec<u8>, String> {
        let (key_type, key_data) = self
            .split_once(':')
            .ok_or_else(|| "Invalid key format".to_string())?;

        let bytes = bs58::decode(key_data)
            .into_vec()
            .map_err(|e| format!("Failed to decode base58: {}", e))?;

        match key_type {
            "ed25519" => {
                if bytes.len() == 32 {
                    Ok(bytes)
                } else {
                    Err("ED25519 public key should be 32 bytes long".to_string())
                }
            }
            "secp256k1" => {
                if bytes.len() == 64 {
                    Ok(bytes)
                } else {
                    Err("SECP256K1 public key should be 64 bytes long".to_string())
                }
            }
            _ => Err("Unknown key type".into()),
        }
    }
}

/// Helper function to decode a base58 string into a fixed-size byte array.
fn decode_base58_to_fixed_bytes<const N: usize>(input: &str) -> Result<[u8; N], String> {
    bs58::decode(input)
        .into_vec()
        .map_err(|e| format!("Failed to decode base58: {}", e))
        .and_then(|bytes| {
            bytes
                .try_into()
                .map_err(|_| format!("Expected {} bytes", N))
        })
}

#[cfg(test)]
mod tests {
    use crate::constants::{ED25519_PUBLIC_KEY_LENGTH, SECP256K1_PUBLIC_KEY_LENGTH};

    use super::*;

    #[test]
    fn test_to_public_key_ed25519() {
        let key_str = "ed25519:6E8sCci9badyRkXb3JoRpBj5p8C6Tw41ELDZoiihKEtp";
        let public_key = key_str.to_public_key();

        assert!(public_key.is_ok());

        match public_key.unwrap() {
            PublicKey::ED25519(public_key) => {
                assert_eq!(public_key.0.len(), ED25519_PUBLIC_KEY_LENGTH)
            }
            _ => panic!("Expected ED25519 key"),
        }
    }

    #[test]
    fn test_to_public_key_secp256k1() {
        let key_str = "secp256k1:3bTpKQ4f3xW1H5VkJrPSLffYiw5XwKMyRsfEqQViakTkUG9N5U2HqfpT3UGsJ93cRURdEYfA4J4wmdLcsUEnT7wx";
        let public_key = key_str.to_public_key();

        println!("{:?}", public_key);

        assert!(public_key.is_ok());

        match public_key.unwrap() {
            PublicKey::SECP256K1(public_key) => {
                assert_eq!(public_key.0.len(), SECP256K1_PUBLIC_KEY_LENGTH)
            }
            _ => panic!("Expected SECP256K1 key"),
        }
    }

    #[test]
    fn test_to_public_key_as_bytes_ed25519() {
        let key_str = "ed25519:6E8sCci9badyRkXb3JoRpBj5p8C6Tw41ELDZoiihKEtp";
        let public_key_bytes = key_str.to_public_key_as_bytes();

        assert!(public_key_bytes.is_ok());
        assert_eq!(public_key_bytes.unwrap().len(), ED25519_PUBLIC_KEY_LENGTH);
    }

    #[test]
    fn test_to_public_key_as_bytes_secp256k1() {
        let key_str = "secp256k1:3bTpKQ4f3xW1H5VkJrPSLffYiw5XwKMyRsfEqQViakTkUG9N5U2HqfpT3UGsJ93cRURdEYfA4J4wmdLcsUEnT7wx";
        let public_key_bytes = key_str.to_public_key_as_bytes();

        assert!(public_key_bytes.is_ok());
        assert_eq!(public_key_bytes.unwrap().len(), SECP256K1_PUBLIC_KEY_LENGTH);
    }

    #[test]
    fn test_invalid_key_format() {
        let key_str = "invalid:6E8sCci9badyRkXb3JoRpBj5p8C6Tw41ELDZoiihKEtp";
        let public_key = key_str.to_public_key();

        assert!(public_key.is_err());
    }

    #[test]
    fn test_invalid_base58() {
        let key_str = "ed25519:invalidbase58";
        let public_key = key_str.to_public_key();

        assert!(public_key.is_err());
    }

    #[test]
    fn test_invalid_key_length_ed25519() {
        let key_str = "ed25519:abcd"; // Too short for a valid ED25519 key
        let public_key_bytes = key_str.to_public_key_as_bytes();

        assert!(public_key_bytes.is_err());
    }

    #[test]
    fn test_invalid_key_length_secp256k1() {
        let key_str = "secp256k1:abcd"; // Too short for a valid SECP256K1 key
        let public_key_bytes = key_str.to_public_key_as_bytes();

        assert!(public_key_bytes.is_err());
    }
}
