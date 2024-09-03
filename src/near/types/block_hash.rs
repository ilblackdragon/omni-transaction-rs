use borsh::BorshSerialize;
use near_sdk::serde::{Deserialize, Deserializer, Serialize};
use serde::de;
use serde_big_array::BigArray;

#[derive(Serialize, Debug, Clone, BorshSerialize, PartialEq, Eq)]
#[serde(crate = "near_sdk::serde")]
pub struct BlockHash(#[serde(with = "BigArray")] pub [u8; 32]);

impl From<[u8; 32]> for BlockHash {
    fn from(data: [u8; 32]) -> Self {
        Self(data)
    }
}

impl<'de> Deserialize<'de> for BlockHash {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct BlockHashOrBytes;

        impl<'de> serde::de::Visitor<'de> for BlockHashOrBytes {
            type Value = BlockHash;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a string or byte array representing a block hash")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                let bytes = bs58::decode(value).into_vec().map_err(de::Error::custom)?;
                let array: [u8; 32] = bytes
                    .try_into()
                    .map_err(|_| de::Error::custom("Invalid length"))?;

                Ok(BlockHash(array))
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                let mut arr = [0u8; 32];
                for i in 0..32 {
                    arr[i] = seq
                        .next_element()?
                        .ok_or_else(|| de::Error::invalid_length(i, &self))?;
                }
                Ok(BlockHash(arr))
            }

            fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
            where
                V: de::MapAccess<'de>,
            {
                let key = map
                    .next_key::<String>()?
                    .ok_or_else(|| de::Error::missing_field("block_hash"))?;

                if key.as_str() != "block_hash" {
                    return Err(de::Error::unknown_field(&key, &["block_hash"]));
                }

                let bytes: Vec<u8> = map.next_value()?;

                if bytes.len() != 32 {
                    return Err(de::Error::invalid_length(bytes.len(), &"32"));
                }

                let mut arr = [0u8; 32];
                arr.copy_from_slice(&bytes);
                Ok(BlockHash(arr))
            }
        }

        deserializer.deserialize_any(BlockHashOrBytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::serde_json;

    #[test]
    fn test_blockhash_deserialize_from_bytes() {
        let bytes: [u8; 32] = [1; 32];
        let bytes_as_json: String = serde_json::to_string(&bytes).unwrap();

        let block_hash: BlockHash = serde_json::from_str(&bytes_as_json).unwrap();

        assert_eq!(block_hash.0, bytes);
    }

    #[test]
    fn test_blockhash_deserialize_from_json_string() {
        let json = r#"{
            "block_hash": [57, 74, 190, 179, 94, 112, 118, 9, 222, 143, 115, 182, 61, 67, 189, 26, 55, 
            111, 254, 103, 147, 92, 170, 104, 147, 125, 210, 155, 192, 78, 103, 60]
        }"#;

        let block_hash: BlockHash = serde_json::from_str(json).unwrap();

        assert_eq!(block_hash.0.len(), 32);
        assert_eq!(block_hash.0[0], 57);
        assert_eq!(block_hash.0[31], 60);
    }

    #[test]
    fn test_blockhash_deserialize_from_base58() {
        let base58 = "CjNSmWXTWhC3EhRVtqLhRmWMTkRbU96wUACqxMtV1uGf";
        let json = format!("\"{}\"", base58);

        let block_hash: BlockHash = serde_json::from_str(&json).unwrap();

        assert_eq!(block_hash.0.len(), 32);
    }

    #[test]
    fn test_blockhash_serialize_deserialize_roundtrip() {
        let original = BlockHash([2; 32]);
        let serialized = serde_json::to_string(&original).unwrap();

        let deserialized: BlockHash = serde_json::from_str(&serialized).unwrap();

        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_blockhash_from_invalid_base58() {
        let invalid_base58 = "\"invalid_base58_string\"";

        assert!(serde_json::from_str::<BlockHash>(invalid_base58).is_err());
    }
}
