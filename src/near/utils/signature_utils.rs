use crate::{
    constants::{ED25519_SIGNATURE_LENGTH, SECP256K1_SIGNATURE_LENGTH},
    near::types::{ED25519Signature, Secp256K1Signature, Signature},
};
use bs58;
use std::convert::TryInto;

pub trait SignatureStrExt {
    fn to_signature(&self) -> Result<Signature, String>;
    fn to_signature_as_bytes(&self) -> Result<Vec<u8>, String>;
    fn try_ed25519_into_bytes(&self) -> Result<[u8; ED25519_SIGNATURE_LENGTH], String>;
    fn try_secp256k1_into_bytes(&self) -> Result<[u8; SECP256K1_SIGNATURE_LENGTH], String>;
    fn to_ed25519_signature(&self) -> Result<ED25519Signature, String>;
    fn to_secp256k1_signature(&self) -> Result<Secp256K1Signature, String>;
}

impl SignatureStrExt for str {
    fn to_signature(&self) -> Result<Signature, String> {
        let bytes = self.to_signature_as_bytes()?;
        if self.starts_with("ed25519:") {
            Ok(Signature::ED25519(ED25519Signature {
                r: bytes[..32]
                    .try_into()
                    .map_err(|_| "Invalid length for ED25519 signature".to_string())?,
                s: bytes[32..]
                    .try_into()
                    .map_err(|_| "Invalid length for ED25519 signature".to_string())?,
            }))
        } else if self.starts_with("secp256k1:") {
            Ok(Signature::SECP256K1(Secp256K1Signature(
                bytes
                    .try_into()
                    .map_err(|_| "Invalid length for SECP256K1 signature".to_string())?,
            )))
        } else {
            Err("Unknown key type".into())
        }
    }

    fn to_signature_as_bytes(&self) -> Result<Vec<u8>, String> {
        let (key_type, key_data) = self
            .split_once(':')
            .ok_or_else(|| "Invalid key format".to_string())?;

        let bytes = bs58::decode(key_data)
            .into_vec()
            .map_err(|e| format!("Failed to decode base58: {}", e))?;

        match key_type {
            "ed25519" => {
                if bytes.len() == ED25519_SIGNATURE_LENGTH {
                    Ok(bytes)
                } else {
                    Err("ED25519 public key should be 32 bytes long".to_string())
                }
            }
            "secp256k1" => {
                if bytes.len() == SECP256K1_SIGNATURE_LENGTH {
                    Ok(bytes)
                } else {
                    Err("SECP256K1 public key should be 64 bytes long".to_string())
                }
            }
            _ => Err("Unknown key type".into()),
        }
    }

    fn try_ed25519_into_bytes(&self) -> Result<[u8; ED25519_SIGNATURE_LENGTH], String> {
        let bytes = self.to_signature_as_bytes()?;
        if bytes.len() == ED25519_SIGNATURE_LENGTH {
            Ok(bytes.try_into().unwrap())
        } else {
            Err("Invalid length for ED25519 signature".to_string())
        }
    }

    fn try_secp256k1_into_bytes(&self) -> Result<[u8; SECP256K1_SIGNATURE_LENGTH], String> {
        let bytes = self.to_signature_as_bytes()?;
        if bytes.len() == SECP256K1_SIGNATURE_LENGTH {
            Ok(bytes.try_into().unwrap())
        } else {
            Err("Invalid length for SECP256K1 signature".to_string())
        }
    }

    fn to_ed25519_signature(&self) -> Result<ED25519Signature, String> {
        let bytes = self.try_ed25519_into_bytes()?;
        Ok(ED25519Signature {
            r: bytes[..32]
                .try_into()
                .map_err(|_| "Invalid length for r".to_string())?,
            s: bytes[32..]
                .try_into()
                .map_err(|_| "Invalid length for s".to_string())?,
        })
    }

    fn to_secp256k1_signature(&self) -> Result<Secp256K1Signature, String> {
        let bytes = self.try_secp256k1_into_bytes()?;
        Ok(Secp256K1Signature(bytes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_signature_for_ed25519() {
        let signature = "ed25519:3s1dvZdQtcAjBksMHFrysqvF63wnyMHPA4owNQmCJZ2EBakZEKdtMsLqrHdKWQjJbSRN6kRknN2WdwSBLWGCokXj";
        let parsed_signature = signature.to_signature().unwrap();

        assert_eq!(
            parsed_signature,
            Signature::ED25519(ED25519Signature {
                r: [
                    143, 41, 92, 68, 235, 139, 203, 41, 13, 46, 193, 146, 56, 12, 149, 33, 210,
                    230, 203, 72, 19, 46, 133, 131, 167, 195, 248, 195, 112, 92, 29, 80
                ],
                s: [
                    36, 222, 15, 68, 225, 165, 228, 104, 54, 185, 125, 74, 30, 36, 246, 87, 37,
                    117, 254, 13, 14, 76, 65, 232, 79, 55, 50, 184, 64, 151, 92, 2
                ],
            })
        );
    }

    #[test]
    fn test_to_signature_for_secp256k1() {
        let signature = "secp256k1:5N5CB9H1dmB9yraLGCo4ZCQTcF24zj4v2NT14MHdH3aVhRoRXrX3AhprHr2w6iXNBZDmjMS1Ntzjzq8Bv6iBvwth6";
        let parsed_signature = signature.to_signature().unwrap();

        assert_eq!(
            parsed_signature,
            Signature::SECP256K1(Secp256K1Signature([
                49, 113, 234, 139, 121, 242, 231, 74, 231, 227, 151, 63, 4, 67, 229, 181, 94, 121,
                2, 93, 63, 105, 108, 125, 132, 105, 151, 117, 239, 187, 118, 202, 122, 231, 222,
                54, 67, 136, 211, 190, 147, 156, 12, 179, 103, 154, 245, 148, 66, 243, 52, 30, 190,
                114, 76, 13, 85, 239, 79, 37, 112, 230, 203, 114, 1
            ]))
        );
    }

    #[test]
    fn test_to_signature_for_invalid_signature() {
        let signature = "invalid:3s1dvZdQtcAjBksMHFrysqvF63wnyMHPA4owNQmCJZ2EBakZEKdtMsLqrHdKWQjJbSRN6kRknN2WdwSBLWGCokXj";
        assert!(signature.to_signature().is_err());
    }

    #[test]
    fn test_to_signature_for_invalid_signature_length() {
        let signature = "ed25519:3s1dvZ";
        assert!(signature.to_signature().is_err());

        let signature = "secp256k1:5N5CB9H1dBv6iBh6";
        assert!(signature.to_signature().is_err());
    }

    #[test]
    fn test_try_ed25519_into_bytes() {
        let signature = "ed25519:3s1dvZdQtcAjBksMHFrysqvF63wnyMHPA4owNQmCJZ2EBakZEKdtMsLqrHdKWQjJbSRN6kRknN2WdwSBLWGCokXj";
        let bytes = signature.try_ed25519_into_bytes().unwrap();

        assert_eq!(
            bytes,
            [
                143, 41, 92, 68, 235, 139, 203, 41, 13, 46, 193, 146, 56, 12, 149, 33, 210, 230,
                203, 72, 19, 46, 133, 131, 167, 195, 248, 195, 112, 92, 29, 80, 36, 222, 15, 68,
                225, 165, 228, 104, 54, 185, 125, 74, 30, 36, 246, 87, 37, 117, 254, 13, 14, 76,
                65, 232, 79, 55, 50, 184, 64, 151, 92, 2
            ]
        );
    }

    #[test]
    fn test_try_secp256k1_into_bytes() {
        let signature = "secp256k1:5N5CB9H1dmB9yraLGCo4ZCQTcF24zj4v2NT14MHdH3aVhRoRXrX3AhprHr2w6iXNBZDmjMS1Ntzjzq8Bv6iBvwth6";
        let bytes = signature.try_secp256k1_into_bytes().unwrap();

        assert_eq!(
            bytes,
            [
                49, 113, 234, 139, 121, 242, 231, 74, 231, 227, 151, 63, 4, 67, 229, 181, 94, 121,
                2, 93, 63, 105, 108, 125, 132, 105, 151, 117, 239, 187, 118, 202, 122, 231, 222,
                54, 67, 136, 211, 190, 147, 156, 12, 179, 103, 154, 245, 148, 66, 243, 52, 30, 190,
                114, 76, 13, 85, 239, 79, 37, 112, 230, 203, 114, 1
            ]
        );
    }

    #[test]
    fn test_to_ed25519_signature() {
        let signature = "ed25519:3s1dvZdQtcAjBksMHFrysqvF63wnyMHPA4owNQmCJZ2EBakZEKdtMsLqrHdKWQjJbSRN6kRknN2WdwSBLWGCokXj";
        let parsed_signature = signature.to_ed25519_signature().unwrap();

        assert_eq!(
            parsed_signature,
            ED25519Signature {
                r: [
                    143, 41, 92, 68, 235, 139, 203, 41, 13, 46, 193, 146, 56, 12, 149, 33, 210,
                    230, 203, 72, 19, 46, 133, 131, 167, 195, 248, 195, 112, 92, 29, 80
                ],
                s: [
                    36, 222, 15, 68, 225, 165, 228, 104, 54, 185, 125, 74, 30, 36, 246, 87, 37,
                    117, 254, 13, 14, 76, 65, 232, 79, 55, 50, 184, 64, 151, 92, 2
                ],
            }
        );
    }

    #[test]
    fn test_to_secp256k1_signature() {
        let signature = "secp256k1:5N5CB9H1dmB9yraLGCo4ZCQTcF24zj4v2NT14MHdH3aVhRoRXrX3AhprHr2w6iXNBZDmjMS1Ntzjzq8Bv6iBvwth6";
        let parsed_signature = signature.to_secp256k1_signature().unwrap();

        assert_eq!(
            parsed_signature,
            Secp256K1Signature([
                49, 113, 234, 139, 121, 242, 231, 74, 231, 227, 151, 63, 4, 67, 229, 181, 94, 121,
                2, 93, 63, 105, 108, 125, 132, 105, 151, 117, 239, 187, 118, 202, 122, 231, 222,
                54, 67, 136, 211, 190, 147, 156, 12, 179, 103, 154, 245, 148, 66, 243, 52, 30, 190,
                114, 76, 13, 85, 239, 79, 37, 112, 230, 203, 114, 1
            ])
        );
    }
}
