use borsh::{BorshDeserialize, BorshSerialize};
use bs58;
use serde::{Deserialize, Deserializer};

use crate::constants::{COMPONENT_SIZE, SECP256K1_SIGNATURE_LENGTH};

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub enum Signature {
    ED25519(ED25519Signature),
    SECP256K1(Secp256K1Signature),
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct ED25519Signature {
    pub r: ComponentBytes,
    pub s: ComponentBytes,
}

/// Size of an `R` or `s` component of an Ed25519 signature when serialized as bytes.
pub type ComponentBytes = [u8; COMPONENT_SIZE];

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct Secp256K1Signature(pub [u8; SECP256K1_SIGNATURE_LENGTH]);

impl<'de> Deserialize<'de> for Signature {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: String = Deserialize::deserialize(deserializer)?;
        let (key_type, sig_data) = s.split_at(
            s.find(':')
                .ok_or_else(|| serde::de::Error::custom("Invalid signature format"))?,
        );
        let sig_data = &sig_data[1..]; // Skip the colon

        match key_type {
            "ed25519" => {
                let bytes = bs58::decode(sig_data)
                    .into_vec()
                    .map_err(serde::de::Error::custom)?;
                let signature = ED25519Signature {
                    r: bytes[0..COMPONENT_SIZE]
                        .try_into()
                        .map_err(serde::de::Error::custom)?,
                    s: bytes[COMPONENT_SIZE..]
                        .try_into()
                        .map_err(serde::de::Error::custom)?,
                };
                Ok(Self::ED25519(signature))
            }
            "secp256k1" => {
                let bytes = bs58::decode(sig_data)
                    .into_vec()
                    .map_err(serde::de::Error::custom)?;
                if bytes.len() != 65 {
                    return Err(serde::de::Error::custom(
                        "Invalid SECP256K1 signature length",
                    ));
                }
                let mut array = [0u8; 65];
                array.copy_from_slice(&bytes);
                Ok(Self::SECP256K1(Secp256K1Signature(array)))
            }
            _ => Err(serde::de::Error::custom("Unknown key type")),
        }
    }
}

// #[cfg(test)]
// mod tests {
//     #[test]
//     fn test_deserialize_signature() {}
// }
