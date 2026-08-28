use core::fmt;
use std::str::FromStr;

#[cfg(feature = "serde")]
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[cfg(feature = "serde")]
use super::address::Base58Visitor;
use crate::constants::ED25519_SIGNATURE_LENGTH;
use crate::solana::utils::decode_base58_fixed;

/// A raw 64-byte ed25519 signature over the serialized message bytes.
///
/// Solana transaction signatures are *always* ed25519 (never secp256k1): the
/// signer's public key is `account_keys[i]` of the message and the signed
/// payload is the exact output of
/// [`SolanaTransaction::build_for_signing`](crate::solana::SolanaTransaction::build_for_signing)
/// with no additional hashing.
///
/// Conventionally displayed (and serialized to JSON) as a base58 string; the
/// transaction id of a Solana transaction is `base58(signature[0])`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SolanaSignature(pub [u8; ED25519_SIGNATURE_LENGTH]);

impl SolanaSignature {
    /// Parses a signature from its base58 string representation.
    pub fn from_base58(s: &str) -> Result<Self, String> {
        decode_base58_fixed::<ED25519_SIGNATURE_LENGTH>(s).map(Self)
    }

    /// Returns the base58 string representation of the signature.
    pub fn to_base58(&self) -> String {
        bs58::encode(&self.0).into_string()
    }

    /// Returns the raw 64 bytes of the signature.
    pub const fn to_bytes(&self) -> [u8; ED25519_SIGNATURE_LENGTH] {
        self.0
    }
}

impl From<[u8; ED25519_SIGNATURE_LENGTH]> for SolanaSignature {
    fn from(bytes: [u8; ED25519_SIGNATURE_LENGTH]) -> Self {
        Self(bytes)
    }
}

impl FromStr for SolanaSignature {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_base58(s)
    }
}

impl fmt::Display for SolanaSignature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_base58())
    }
}

// Hand-implemented (instead of derived with `serde-big-array`) so the JSON
// form is the canonical base58 string every Solana tool uses; deserialization
// also accepts a 64-byte array.
#[cfg(feature = "serde")]
impl Serialize for SolanaSignature {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_base58())
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for SolanaSignature {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer
            .deserialize_any(Base58Visitor::<64>("base58 string or 64-byte array"))
            .map(Self)
    }
}

// Hand-implemented as a string schema to match the base58 serde
// representation above (the derive would describe a 64-element array), same
// pattern as the NEAR `Secp256K1Signature` type of this crate.
#[cfg(feature = "schemars")]
impl schemars::JsonSchema for SolanaSignature {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "SolanaSignature".into()
    }

    fn json_schema(generator: &mut schemars::generate::SchemaGenerator) -> schemars::Schema {
        <String>::json_schema(generator)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIGNATURE_HEX: &str = "a8038ea08a399e279d692b0a5710e60f9dbe8781f9b0cf444d8be892126a5a9893097f056a88c75b126ad9ea78a5133be308b8bc46c424ab912d93e6e3273c0b";

    fn signature_bytes() -> [u8; 64] {
        hex::decode(SIGNATURE_HEX).unwrap().try_into().unwrap()
    }

    #[test]
    fn test_signature_base58_round_trip() {
        let signature = SolanaSignature(signature_bytes());
        let base58 = signature.to_base58();
        let parsed = SolanaSignature::from_base58(&base58).unwrap();
        assert_eq!(parsed, signature);
        assert_eq!(parsed.to_bytes(), signature_bytes());
        assert_eq!(signature.to_string(), base58);
    }

    #[test]
    fn test_signature_from_invalid_base58() {
        assert!(SolanaSignature::from_base58("abc").is_err());
        assert!(SolanaSignature::from_base58("not-base58-0OIl").is_err());
    }

    /// A byte-array form of the wrong length must be rejected by the visitor
    /// itself, with an error naming the expected length (see the equivalent
    /// address test for why the data format is not relied on for that).
    #[test]
    #[cfg(feature = "serde_json")]
    fn test_signature_serde_rejects_wrong_length_byte_array() {
        let json_of = |count: u16| {
            let elements = (0..count)
                .map(|i| (i % 256).to_string())
                .collect::<Vec<_>>();
            format!("[{}]", elements.join(","))
        };
        assert!(serde_json::from_str::<SolanaSignature>(&json_of(64)).is_ok());

        let err = serde_json::from_str::<SolanaSignature>(&json_of(65)).unwrap_err();
        assert!(
            err.to_string().contains("expected exactly 64 bytes"),
            "unexpected error: {err}"
        );
        assert!(serde_json::from_str::<SolanaSignature>(&json_of(80)).is_err());
        assert!(serde_json::from_str::<SolanaSignature>(&json_of(63)).is_err());
    }

    #[test]
    #[cfg(feature = "serde_json")]
    fn test_signature_serde_round_trip() {
        let signature = SolanaSignature(signature_bytes());
        let json = serde_json::to_string(&signature).unwrap();
        assert_eq!(json, format!("\"{}\"", signature.to_base58()));
        let parsed: SolanaSignature = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, signature);

        // Byte-array form is accepted too.
        let bytes_json = serde_json::to_string(&signature_bytes().to_vec()).unwrap();
        let parsed_from_bytes: SolanaSignature = serde_json::from_str(&bytes_json).unwrap();
        assert_eq!(parsed_from_bytes, signature);
    }
}
