use crate::constants::{ED25519_PUBLIC_KEY_LENGTH, SECP256K1_PUBLIC_KEY_LENGTH};
use crate::near::utils::PublicKeyStrExt;
use borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::serde::{Deserialize, Deserializer, Serialize};
use serde::de;
use serde_big_array::BigArray;
use std::io::{Error, Write};

#[derive(Serialize, Deserialize, BorshDeserialize, PartialEq, Eq, Debug, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct Secp256K1PublicKey(#[serde(with = "BigArray")] pub [u8; SECP256K1_PUBLIC_KEY_LENGTH]);

#[derive(Serialize, Deserialize, BorshDeserialize, PartialEq, Eq, Debug, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct ED25519PublicKey(pub [u8; ED25519_PUBLIC_KEY_LENGTH]);

#[derive(Serialize, PartialEq, Eq, Debug, Clone)]
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

impl BorshDeserialize for PublicKey {
    fn deserialize(buf: &mut &[u8]) -> std::io::Result<Self> {
        let key_type = <u8 as BorshDeserialize>::deserialize(buf)?;
        match key_type {
            0 => Ok(PublicKey::ED25519(
                <ED25519PublicKey as BorshDeserialize>::deserialize(buf)?,
            )),
            1 => Ok(PublicKey::SECP256K1(
                <Secp256K1PublicKey as BorshDeserialize>::deserialize(buf)?,
            )),
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid public key type",
            )),
        }
    }

    fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
        let key_type = u8::deserialize_reader(reader)?;
        match key_type {
            0 => Ok(PublicKey::ED25519(ED25519PublicKey::deserialize_reader(
                reader,
            )?)),
            1 => Ok(PublicKey::SECP256K1(
                Secp256K1PublicKey::deserialize_reader(reader)?,
            )),
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid public key type",
            )),
        }
    }
}

// From implementations for fixed size arrays
impl From<[u8; SECP256K1_PUBLIC_KEY_LENGTH]> for Secp256K1PublicKey {
    fn from(data: [u8; SECP256K1_PUBLIC_KEY_LENGTH]) -> Self {
        Self(data)
    }
}

impl From<[u8; ED25519_PUBLIC_KEY_LENGTH]> for ED25519PublicKey {
    fn from(data: [u8; ED25519_PUBLIC_KEY_LENGTH]) -> Self {
        Self(data)
    }
}

// TryFrom implementations for slices and vectors
impl TryFrom<&[u8]> for PublicKey {
    type Error = String;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        match value.len() {
            ED25519_PUBLIC_KEY_LENGTH => Ok(PublicKey::ED25519(ED25519PublicKey(
                value.try_into().unwrap(),
            ))),
            SECP256K1_PUBLIC_KEY_LENGTH => Ok(PublicKey::SECP256K1(Secp256K1PublicKey(
                value.try_into().unwrap(),
            ))),
            _ => Err("Invalid public key length".to_string()),
        }
    }
}

impl TryFrom<Vec<u8>> for PublicKey {
    type Error = String;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        Self::try_from(value.as_slice())
    }
}

// Serialization
impl<'de> Deserialize<'de> for PublicKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct PublicKeyOrBytes;

        impl<'de> serde::de::Visitor<'de> for PublicKeyOrBytes {
            type Value = PublicKey;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a string or byte array representing a public key")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                value.to_public_key().map_err(de::Error::custom)
            }

            fn visit_map<V>(self, mut map: V) -> Result<PublicKey, V::Error>
            where
                V: de::MapAccess<'de>,
            {
                let key = map
                    .next_key::<String>()?
                    .ok_or_else(|| de::Error::missing_field("key type"))?;
                match key.as_str() {
                    "ED25519" => {
                        let bytes: Vec<u8> = map.next_value()?;
                        PublicKey::try_from(bytes.as_slice()).map_err(de::Error::custom)
                    }
                    "SECP256K1" => {
                        let bytes: Vec<u8> = map.next_value()?;
                        PublicKey::try_from(bytes.as_slice()).map_err(de::Error::custom)
                    }
                    _ => Err(de::Error::unknown_field(&key, &["ED25519", "SECP256K1"])),
                }
            }
        }

        deserializer.deserialize_any(PublicKeyOrBytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use borsh;
    use near_sdk::serde_json;

    #[test]
    fn test_public_key_serialization() {
        let ed25519_key = PublicKey::ED25519(ED25519PublicKey([3; ED25519_PUBLIC_KEY_LENGTH]));
        let secp256k1_key =
            PublicKey::SECP256K1(Secp256K1PublicKey([4; SECP256K1_PUBLIC_KEY_LENGTH]));

        let keys = vec![ed25519_key, secp256k1_key];

        for key in keys {
            let serialized = serde_json::to_string(&key).unwrap();

            let deserialized: PublicKey = serde_json::from_str(&serialized).unwrap();

            assert_eq!(key, deserialized);
        }
    }

    #[test]
    fn test_public_key_from_str() {
        let ed25519_str = "ed25519:2S87aQ1PM9o6eBcEXnTR5yBAVRTiNmvj8J8ngZ6FzSca";
        let secp256k1_str = "secp256k1:3bTpKQ4f3xW1H5VkJrPSLffYiw5XwKMyRsfEqQViakTkUG9N5U2HqfpT3UGsJ93cRURdEYfA4J4wmdLcsUEnT7wx";

        let ed25519_key = ed25519_str.to_public_key().unwrap();
        let secp256k1_key = secp256k1_str.to_public_key().unwrap();

        assert!(matches!(ed25519_key, PublicKey::ED25519(_)));
        assert!(matches!(secp256k1_key, PublicKey::SECP256K1(_)));
    }

    #[test]
    fn test_borsh_serialization() {
        let public_key = PublicKey::ED25519(ED25519PublicKey([6; ED25519_PUBLIC_KEY_LENGTH]));
        let serialized = borsh::to_vec(&public_key).expect("Failed to serialize PublicKey");

        assert_eq!(serialized[0], 0); // ED25519 type
        assert_eq!(&serialized[1..], &[6; ED25519_PUBLIC_KEY_LENGTH]);
    }

    #[test]
    fn test_public_key_borsh_serialization() {
        let ed25519_key = PublicKey::ED25519(ED25519PublicKey([6; ED25519_PUBLIC_KEY_LENGTH]));
        let secp256k1_key =
            PublicKey::SECP256K1(Secp256K1PublicKey([7; SECP256K1_PUBLIC_KEY_LENGTH]));

        for key in vec![ed25519_key, secp256k1_key] {
            let serialized = borsh::to_vec(&key).expect("Failed to serialize PublicKey");
            let deserialized =
                PublicKey::try_from_slice(&serialized).expect("Failed to deserialize PublicKey");
            assert_eq!(key, deserialized);

            match key {
                PublicKey::ED25519(_) => {
                    assert_eq!(serialized[0], 0);
                    assert_eq!(serialized.len(), 1 + ED25519_PUBLIC_KEY_LENGTH);
                }
                PublicKey::SECP256K1(_) => {
                    assert_eq!(serialized[0], 1);
                    assert_eq!(serialized.len(), 1 + SECP256K1_PUBLIC_KEY_LENGTH);
                }
            }
        }
    }

    #[test]
    fn test_public_key_serde_json_serialization() {
        let ed25519_key = PublicKey::ED25519(ED25519PublicKey([8; ED25519_PUBLIC_KEY_LENGTH]));
        let secp256k1_key =
            PublicKey::SECP256K1(Secp256K1PublicKey([9; SECP256K1_PUBLIC_KEY_LENGTH]));

        for key in vec![ed25519_key, secp256k1_key] {
            let serialized =
                serde_json::to_string(&key).expect("Failed to serialize PublicKey to JSON");
            let deserialized: PublicKey = serde_json::from_str(&serialized)
                .expect("Failed to deserialize PublicKey from JSON");
            assert_eq!(key, deserialized);

            // Check if the JSON string contains the correct key type
            match key {
                PublicKey::ED25519(_) => assert!(serialized.contains("ED25519")),
                PublicKey::SECP256K1(_) => assert!(serialized.contains("SECP256K1")),
            }
        }
    }

    #[test]
    fn test_public_key_from_invalid_str() {
        let invalid_str = "invalid:2S87aQ1PM9o6eBcEXnTR5yBAVRTiNmvj8J8ngZ6FzSca";
        assert!(invalid_str.to_public_key().is_err());
    }

    #[test]
    fn test_public_key_deserialize_from_near_tx() {
        let json = r#"
            {
              "signer_public_key": {
                "ED25519": [
                  77, 167, 224, 244, 9, 106, 175, 44, 229, 94, 55, 22, 87, 205, 48, 137,
                  186, 30, 159, 89, 244, 214, 226, 123, 208, 46, 71, 42, 22, 166, 29, 193
                ]
              }
            }"#;

        let parsed: serde_json::Value = serde_json::from_str(json).unwrap();
        let public_key: PublicKey =
            serde_json::from_value(parsed["signer_public_key"].clone()).unwrap();

        match public_key {
            PublicKey::ED25519(key) => {
                assert_eq!(key.0.len(), 32);
                assert_eq!(key.0[0], 77);
                assert_eq!(key.0[31], 193);
            }
            _ => panic!("Expected ED25519 public key"),
        }
    }
}
