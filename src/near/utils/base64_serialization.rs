/// Convenience module to allow annotating a serde structure as base64 bytes.
use near_sdk::base64::Engine;
use serde::{de, Deserialize, Deserializer, Serializer};

/// Dummy type for `serde_with` compatibility that provides base64 serialization
/// and JSON schema annotations.
#[derive(Debug, Clone, Copy)]
pub struct Base64;

impl Base64 {
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

impl schemars::JsonSchema for Base64 {
    fn schema_name() -> String {
        "String".to_string()
    }

    fn json_schema(_gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        schemars::schema::SchemaObject {
            instance_type: Some(schemars::schema::InstanceType::String.into()),
            format: Some("byte".to_string()),
            ..Default::default()
        }
        .into()
    }

    fn is_referenceable() -> bool {
        false
    }
}
