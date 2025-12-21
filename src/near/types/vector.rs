use borsh::{BorshDeserialize, BorshSerialize};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// TODO: we should remove that in favor of near-sdk::json_types::Base64VecU8
// And I'm not sure if we need to put serde by default. We should follow near-sdk path with a schema only for abi

/// Helper class to serialize/deserialize `Vec<u8>` to base64 string.
///
/// # Example
/// ```rust
/// use near_sdk::{json_types::Base64VecU8, near};
///
/// #[near(serializers=[json])]
/// struct NewStruct {
///     field: Base64VecU8,
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, BorshDeserialize, BorshSerialize)]
pub struct Base64VecU8(
    #[serde(
        serialize_with = "base64_bytes::serialize",
        deserialize_with = "base64_bytes::deserialize"
    )]
    pub Vec<u8>,
);

impl From<Vec<u8>> for Base64VecU8 {
    fn from(v: Vec<u8>) -> Self {
        Self(v)
    }
}

impl From<Base64VecU8> for Vec<u8> {
    fn from(v: Base64VecU8) -> Self {
        v.0
    }
}

impl JsonSchema for Base64VecU8 {
    fn schema_name() -> String {
        "Base64VecU8".to_string()
    }

    fn json_schema(gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        <String>::json_schema(gen)
    }
}

/// Convenience module to allow annotating a serde structure as base64 bytes.
mod base64_bytes {
    use super::*;
    use near_sdk::base64::Engine;
    use serde::{de, Deserializer, Serializer};

    pub fn serialize<S>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&near_sdk::base64::engine::general_purpose::STANDARD.encode(bytes))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: String = Deserialize::deserialize(deserializer)?;
        near_sdk::base64::engine::general_purpose::STANDARD
            .decode(s.as_str())
            .map_err(de::Error::custom)
    }
}
