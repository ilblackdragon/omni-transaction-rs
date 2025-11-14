/// Convenience module to allow annotating a serde structure as base64 bytes.
use near_sdk::base64::Engine;
use serde::{de, Deserialize, Deserializer, Serializer};

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
