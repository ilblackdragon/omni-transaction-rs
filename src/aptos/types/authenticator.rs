//! Aptos transaction authenticators (Ed25519 key, signature and
//! `TransactionAuthenticator`).
use core::fmt;

use crate::bcs_encoding::{BcsEncode, BcsWriter};
#[cfg(feature = "schemars")]
use schemars::JsonSchema;
#[cfg(feature = "serde")]
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Byte length of an [`Ed25519PublicKey`].
pub const ED25519_PUBLIC_KEY_LENGTH: usize = 32;

/// Byte length of an [`Ed25519Signature`].
pub const ED25519_SIGNATURE_LENGTH: usize = 64;

/// A 32-byte Ed25519 public key.
///
/// Unlike [`AccountAddress`](super::AccountAddress), inside the authenticator
/// this is BCS-encoded as **length-prefixed** bytes: ULEB128 `0x20` followed
/// by the 32 key bytes (mirroring `aptos-crypto`'s `Ed25519PublicKey`).
///
/// The JSON (serde) representation is a `0x`-prefixed lowercase hex string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Ed25519PublicKey(pub [u8; ED25519_PUBLIC_KEY_LENGTH]);

/// A 64-byte Ed25519 signature.
///
/// Inside the authenticator this is BCS-encoded as **length-prefixed** bytes:
/// ULEB128 `0x40` followed by the 64 signature bytes (mirroring
/// `aptos-crypto`'s `Ed25519Signature`).
///
/// The JSON (serde) representation is a `0x`-prefixed lowercase hex string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Ed25519Signature(pub [u8; ED25519_SIGNATURE_LENGTH]);

impl Ed25519PublicKey {
    /// Creates a public key from raw bytes.
    pub const fn new(bytes: [u8; ED25519_PUBLIC_KEY_LENGTH]) -> Self {
        Self(bytes)
    }

    /// Returns the raw 32-byte representation.
    pub const fn into_inner(self) -> [u8; ED25519_PUBLIC_KEY_LENGTH] {
        self.0
    }

    /// Parses a public key from a hex string, with or without a `0x` prefix.
    ///
    /// # Errors
    ///
    /// Returns [`KeyBytesParseError`] unless the input is exactly 64 hex
    /// characters.
    pub fn from_hex(hex_str: &str) -> Result<Self, KeyBytesParseError> {
        let mut bytes = [0u8; ED25519_PUBLIC_KEY_LENGTH];
        decode_exact_hex(hex_str, &mut bytes)?;
        Ok(Self(bytes))
    }

    /// Renders the key as a `0x`-prefixed hex string.
    pub fn to_hex(&self) -> String {
        format!("0x{}", hex::encode(self.0))
    }
}

impl Ed25519Signature {
    /// Creates a signature from raw bytes.
    pub const fn new(bytes: [u8; ED25519_SIGNATURE_LENGTH]) -> Self {
        Self(bytes)
    }

    /// Returns the raw 64-byte representation.
    pub const fn into_inner(self) -> [u8; ED25519_SIGNATURE_LENGTH] {
        self.0
    }

    /// Parses a signature from a hex string, with or without a `0x` prefix.
    ///
    /// # Errors
    ///
    /// Returns [`KeyBytesParseError`] unless the input is exactly 128 hex
    /// characters.
    pub fn from_hex(hex_str: &str) -> Result<Self, KeyBytesParseError> {
        let mut bytes = [0u8; ED25519_SIGNATURE_LENGTH];
        decode_exact_hex(hex_str, &mut bytes)?;
        Ok(Self(bytes))
    }

    /// Renders the signature as a `0x`-prefixed hex string.
    pub fn to_hex(&self) -> String {
        format!("0x{}", hex::encode(self.0))
    }
}

fn decode_exact_hex(hex_str: &str, out: &mut [u8]) -> Result<(), KeyBytesParseError> {
    let hex_str = hex_str.strip_prefix("0x").unwrap_or(hex_str);
    if hex_str.len() != out.len() * 2 {
        return Err(KeyBytesParseError);
    }
    hex::decode_to_slice(hex_str, out).map_err(|_| KeyBytesParseError)
}

impl From<[u8; ED25519_PUBLIC_KEY_LENGTH]> for Ed25519PublicKey {
    fn from(bytes: [u8; ED25519_PUBLIC_KEY_LENGTH]) -> Self {
        Self(bytes)
    }
}

impl From<[u8; ED25519_SIGNATURE_LENGTH]> for Ed25519Signature {
    fn from(bytes: [u8; ED25519_SIGNATURE_LENGTH]) -> Self {
        Self(bytes)
    }
}

impl fmt::Display for Ed25519PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

impl fmt::Display for Ed25519Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

impl BcsEncode for Ed25519PublicKey {
    fn bcs_encode(&self, writer: &mut BcsWriter) {
        writer.write_bytes(&self.0);
    }
}

impl BcsEncode for Ed25519Signature {
    fn bcs_encode(&self, writer: &mut BcsWriter) {
        writer.write_bytes(&self.0);
    }
}

/// Error returned when parsing an Ed25519 key or signature from hex fails.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyBytesParseError;

impl fmt::Display for KeyBytesParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid Ed25519 bytes: expected exact-length hex")
    }
}

impl std::error::Error for KeyBytesParseError {}

// The JSON form is a hex string, not the raw byte array ([u8; 64] also has no
// serde derive support), so serde is implemented by hand. The wire format is
// the hand-rolled BCS above and is never derived from serde.
#[cfg(feature = "serde")]
impl Serialize for Ed25519PublicKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_hex())
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for Ed25519PublicKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let hex_str = String::deserialize(deserializer)?;
        Self::from_hex(&hex_str).map_err(serde::de::Error::custom)
    }
}

#[cfg(feature = "serde")]
impl Serialize for Ed25519Signature {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_hex())
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for Ed25519Signature {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let hex_str = String::deserialize(deserializer)?;
        Self::from_hex(&hex_str).map_err(serde::de::Error::custom)
    }
}

// The schemas mirror the serde form (hex string).
#[cfg(feature = "schemars")]
impl JsonSchema for Ed25519PublicKey {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "Ed25519PublicKey".into()
    }

    fn json_schema(generator: &mut schemars::generate::SchemaGenerator) -> schemars::Schema {
        <String>::json_schema(generator)
    }
}

#[cfg(feature = "schemars")]
impl JsonSchema for Ed25519Signature {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "Ed25519Signature".into()
    }

    fn json_schema(generator: &mut schemars::generate::SchemaGenerator) -> schemars::Schema {
        <String>::json_schema(generator)
    }
}

/// Authenticates a signed Aptos transaction
/// (aptos-core `types/src/transaction/authenticator.rs`).
///
/// BCS encodes the ULEB128 variant index followed by the variant fields.
/// Only `Ed25519` (index 0) is implemented; upstream indices 1..=4
/// (`MultiEd25519`, `MultiAgent`, `FeePayer`, `SingleSender`) are reserved
/// and intentionally not implemented — never reuse them.
///
/// The full `Ed25519` authenticator is exactly 99 bytes:
/// `0x00 || 0x20 || public_key(32) || 0x40 || signature(64)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub enum TransactionAuthenticator {
    /// A single Ed25519 signature (index 0).
    Ed25519 {
        /// The signer's public key.
        public_key: Ed25519PublicKey,
        /// The Ed25519 signature over the transaction signing message.
        signature: Ed25519Signature,
    },
}

impl BcsEncode for TransactionAuthenticator {
    fn bcs_encode(&self, writer: &mut BcsWriter) {
        match self {
            Self::Ed25519 {
                public_key,
                signature,
            } => {
                writer.write_variant(0);
                public_key.bcs_encode(writer);
                signature.bcs_encode(writer);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bcs_encoding::to_bytes;

    #[test]
    fn test_ed25519_authenticator_layout_is_99_bytes() {
        let public_key = Ed25519PublicKey::new([0xaa; 32]);
        let signature = Ed25519Signature::new([0xbb; 64]);
        let authenticator = TransactionAuthenticator::Ed25519 {
            public_key,
            signature,
        };
        let encoded = to_bytes(&authenticator);
        assert_eq!(encoded.len(), 99);
        assert_eq!(encoded[0], 0x00); // variant index
        assert_eq!(encoded[1], 0x20); // key length prefix
        assert_eq!(&encoded[2..34], &[0xaa; 32]);
        assert_eq!(encoded[34], 0x40); // signature length prefix
        assert_eq!(&encoded[35..99], &[0xbb; 64][..]);
    }

    #[test]
    fn test_key_and_signature_are_length_prefixed_like_reference_bcs() {
        // aptos-crypto serializes keys/signatures as variable-length bytes;
        // the reference encoding is bcs of a Vec<u8> with the same content.
        let public_key = Ed25519PublicKey::new([0x01; 32]);
        assert_eq!(
            to_bytes(&public_key),
            bcs::to_bytes(&vec![0x01u8; 32]).unwrap()
        );
        let signature = Ed25519Signature::new([0x02; 64]);
        assert_eq!(
            to_bytes(&signature),
            bcs::to_bytes(&vec![0x02u8; 64]).unwrap()
        );
    }

    #[test]
    fn test_from_hex_requires_exact_length() {
        assert!(Ed25519PublicKey::from_hex(&"11".repeat(32)).is_ok());
        assert!(Ed25519PublicKey::from_hex(&format!("0x{}", "11".repeat(32))).is_ok());
        assert!(Ed25519PublicKey::from_hex(&"11".repeat(31)).is_err());
        assert!(Ed25519PublicKey::from_hex(&"11".repeat(33)).is_err());
        assert!(Ed25519Signature::from_hex(&"22".repeat(64)).is_ok());
        assert!(Ed25519Signature::from_hex(&"22".repeat(63)).is_err());
        assert!(Ed25519Signature::from_hex("zz").is_err());
    }

    #[test]
    #[cfg(feature = "serde_json")]
    fn test_serde_is_hex_string() {
        let public_key = Ed25519PublicKey::new([0xab; 32]);
        let json = serde_json::to_string(&public_key).unwrap();
        assert_eq!(json, format!("\"0x{}\"", "ab".repeat(32)));
        let back: Ed25519PublicKey = serde_json::from_str(&json).unwrap();
        assert_eq!(back, public_key);

        let signature = Ed25519Signature::new([0xcd; 64]);
        let json = serde_json::to_string(&signature).unwrap();
        assert_eq!(json, format!("\"0x{}\"", "cd".repeat(64)));
        let back: Ed25519Signature = serde_json::from_str(&json).unwrap();
        assert_eq!(back, signature);
    }
}
