use borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::serde::{Deserialize, Deserializer, Serialize};
use serde::de;
use serde_big_array::BigArray;

#[derive(Serialize, Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
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
        }

        deserializer.deserialize_any(BlockHashOrBytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use borsh::BorshDeserialize;
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
        let json = r#"[57, 74, 190, 179, 94, 112, 118, 9, 222, 143, 115, 182, 61, 67, 189, 26, 55, 
            111, 254, 103, 147, 92, 170, 104, 147, 125, 210, 155, 192, 78, 103, 60]"#;

        let block_hash: BlockHash = serde_json::from_str(json).unwrap();

        assert_eq!(block_hash.0.len(), 32);
        assert_eq!(block_hash.0[0], 57);
        assert_eq!(block_hash.0[31], 60);
    }

    #[test]
    fn test_blockhash_deserialize_from_json_base58() {
        let base58 = "CjNSmWXTWhC3EhRVtqLhRmWMTkRbU96wUACqxMtV1uGf";

        // Serialize to JSON string
        let json = format!("\"{}\"", base58);

        // Deserialize from JSON string using serde_json
        let block_hash: BlockHash = serde_json::from_str(&json).unwrap();

        assert_eq!(block_hash.0.len(), 32);
        assert_eq!(block_hash.0[0], 174);
        assert_eq!(block_hash.0[31], 252);
    }

    #[test]
    fn test_blockhash_serialize_deserialize_roundtrip() {
        let original = BlockHash([2; 32]);

        // Serialize to JSON string
        let serialized = serde_json::to_string(&original).unwrap();

        // Deserialize from JSON string
        let deserialized: BlockHash = serde_json::from_str(&serialized).unwrap();

        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_blockhash_from_invalid_base58() {
        let invalid_base58 = "\"invalid_base58_string\"";

        assert!(serde_json::from_str::<BlockHash>(invalid_base58).is_err());
    }

    #[test]
    fn test_blockhash_borsh_deserialize_from_bytes() {
        let original_bytes: [u8; 32] = [
            57, 74, 190, 179, 94, 112, 118, 9, 222, 143, 115, 182, 61, 67, 189, 26, 55, 111, 254,
            103, 147, 92, 170, 104, 147, 125, 210, 155, 192, 78, 103, 60,
        ];

        // Create a BlockHash from the original bytes
        let original_block_hash = BlockHash(original_bytes);

        // Serialize the BlockHash using Borsh
        let serialized =
            borsh::to_vec(&original_block_hash).expect("Failed to serialize BlockHash");

        // Deserialize the BlockHash from the serialized bytes
        let deserialized_block_hash = BlockHash::try_from_slice(&serialized).unwrap();

        // Verify that the deserialized BlockHash matches the original
        assert_eq!(deserialized_block_hash.0, original_bytes);
    }

    #[test]
    fn test_blockhash_from_invalid_length() {
        let invalid_json = r#"[1, 2, 3]"#; // Too short
        assert!(serde_json::from_str::<BlockHash>(invalid_json).is_err());
    }

    #[test]
    fn test_blockhash_borsh_roundtrip() {
        let original = BlockHash([3; 32]);

        let serialized = borsh::to_vec(&original).expect("Failed to serialize");
        let deserialized = BlockHash::try_from_slice(&serialized).expect("Failed to deserialize");

        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_blockhash_from_into() {
        let data = [4; 32];
        let block_hash: BlockHash = data.into();

        assert_eq!(block_hash.0, data);
    }
}
