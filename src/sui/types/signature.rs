//! Sui user signature types.
use core::fmt;

use crate::bcs_encoding::{BcsEncode, BcsWriter};
#[cfg(feature = "serde")]
use base64::{engine::general_purpose::STANDARD, Engine};
#[cfg(feature = "schemars")]
use schemars::JsonSchema;
#[cfg(feature = "serde")]
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Byte length of the raw signature component of a [`SuiSignature`].
pub const SUI_SIGNATURE_LENGTH: usize = 64;

/// Byte length of an ed25519 public key.
pub const ED25519_PUBLIC_KEY_LENGTH: usize = 32;

/// Byte length of a compressed secp256k1 / secp256r1 public key.
pub const SECP256K1_PUBLIC_KEY_LENGTH: usize = 33;

/// The signature schemes usable for Sui user signatures, identified on the
/// wire by a one-byte flag.
///
/// Sui defines further flags (`multisig` = `0x03`, `bls` = `0x04`,
/// `zklogin` = `0x05`, `passkey` = `0x06`) that this library does not
/// produce.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub enum SignatureScheme {
    /// Ed25519, flag `0x00`, 32-byte public key. The primary Sui scheme.
    Ed25519,
    /// ECDSA over secp256k1, flag `0x01`, 33-byte compressed public key.
    Secp256k1,
    /// ECDSA over secp256r1, flag `0x02`, 33-byte compressed public key.
    Secp256r1,
}

impl SignatureScheme {
    /// Returns the one-byte wire flag of the scheme.
    pub const fn flag(self) -> u8 {
        match self {
            Self::Ed25519 => 0x00,
            Self::Secp256k1 => 0x01,
            Self::Secp256r1 => 0x02,
        }
    }

    /// Parses a scheme from its wire flag.
    ///
    /// # Errors
    ///
    /// Returns [`SignatureParseError`] for flags this library does not model.
    pub const fn from_flag(flag: u8) -> Result<Self, SignatureParseError> {
        match flag {
            0x00 => Ok(Self::Ed25519),
            0x01 => Ok(Self::Secp256k1),
            0x02 => Ok(Self::Secp256r1),
            _ => Err(SignatureParseError),
        }
    }

    /// Returns the expected public key length of the scheme in bytes.
    pub const fn public_key_length(self) -> usize {
        match self {
            Self::Ed25519 => ED25519_PUBLIC_KEY_LENGTH,
            Self::Secp256k1 | Self::Secp256r1 => SECP256K1_PUBLIC_KEY_LENGTH,
        }
    }
}

/// A serialized Sui user signature: scheme flag, 64-byte signature and the
/// signer's public key.
///
/// # Wire formats — two, do not mix them
///
/// - **JSON-RPC / GraphQL** (`signatures` parameter): the raw concatenation
///   `flag || signature (64 bytes) || public_key` returned by
///   [`Self::to_bytes`] (97 bytes for ed25519, 98 for secp256k1), base64
///   encoded, with **no** length prefix.
/// - **BCS containers** (e.g. the signed transaction envelope built by
///   [`crate::sui::SuiTransaction::build_with_signature`]): the same bytes
///   but ULEB128 length-prefixed, because signatures are historically
///   serialized as BCS `bytes`.
///
/// For secp256k1, `signature` must be the 64-byte `r || s` form with low-`s`
/// normalization and **without** a recovery id: drop the `v` byte returned by
/// recoverable ECDSA signers such as the NEAR MPC before constructing this
/// type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SuiSignature {
    /// The signature scheme of the key that produced `signature`.
    pub scheme: SignatureScheme,
    /// The raw 64-byte signature (`R || s` for ed25519, `r || s` for ECDSA).
    pub signature: [u8; SUI_SIGNATURE_LENGTH],
    /// The signer's public key: 32 bytes for ed25519, 33 compressed bytes
    /// for secp256k1/secp256r1.
    pub public_key: Vec<u8>,
}

impl SuiSignature {
    /// Creates an ed25519 signature (flag `0x00`).
    pub fn ed25519(
        signature: [u8; SUI_SIGNATURE_LENGTH],
        public_key: [u8; ED25519_PUBLIC_KEY_LENGTH],
    ) -> Self {
        Self {
            scheme: SignatureScheme::Ed25519,
            signature,
            public_key: public_key.to_vec(),
        }
    }

    /// Creates a secp256k1 signature (flag `0x01`) from a low-`s` normalized
    /// `r || s` signature and a compressed public key.
    pub fn secp256k1(
        signature: [u8; SUI_SIGNATURE_LENGTH],
        public_key: [u8; SECP256K1_PUBLIC_KEY_LENGTH],
    ) -> Self {
        Self {
            scheme: SignatureScheme::Secp256k1,
            signature,
            public_key: public_key.to_vec(),
        }
    }

    /// Creates a secp256r1 signature (flag `0x02`) from a low-`s` normalized
    /// `r || s` signature and a compressed public key.
    pub fn secp256r1(
        signature: [u8; SUI_SIGNATURE_LENGTH],
        public_key: [u8; SECP256K1_PUBLIC_KEY_LENGTH],
    ) -> Self {
        Self {
            scheme: SignatureScheme::Secp256r1,
            signature,
            public_key: public_key.to_vec(),
        }
    }

    /// Serializes the signature as `flag || signature || public_key`.
    ///
    /// Base64 this value for the `signatures` parameter of
    /// `sui_executeTransactionBlock`; it carries **no** length prefix.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(1 + SUI_SIGNATURE_LENGTH + self.public_key.len());
        bytes.push(self.scheme.flag());
        bytes.extend_from_slice(&self.signature);
        bytes.extend_from_slice(&self.public_key);
        bytes
    }

    /// Parses a signature from its `flag || signature || public_key` form.
    ///
    /// # Errors
    ///
    /// Returns [`SignatureParseError`] on an unknown flag or a length that
    /// does not match the scheme.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, SignatureParseError> {
        let (&flag, rest) = bytes.split_first().ok_or(SignatureParseError)?;
        let scheme = SignatureScheme::from_flag(flag)?;
        if rest.len() != SUI_SIGNATURE_LENGTH + scheme.public_key_length() {
            return Err(SignatureParseError);
        }
        let mut signature = [0u8; SUI_SIGNATURE_LENGTH];
        signature.copy_from_slice(&rest[..SUI_SIGNATURE_LENGTH]);
        Ok(Self {
            scheme,
            signature,
            public_key: rest[SUI_SIGNATURE_LENGTH..].to_vec(),
        })
    }

    /// Renders the signature in the base64 form the Sui JSON-RPC
    /// `signatures` parameter expects.
    #[cfg(feature = "serde")]
    pub fn to_base64(&self) -> String {
        STANDARD.encode(self.to_bytes())
    }

    /// Parses a signature from its base64 JSON-RPC form.
    ///
    /// # Errors
    ///
    /// Returns [`SignatureParseError`] if the input is not valid base64 or
    /// does not decode to a valid serialized signature.
    #[cfg(feature = "serde")]
    pub fn from_base64(base64_str: &str) -> Result<Self, SignatureParseError> {
        let bytes = STANDARD
            .decode(base64_str)
            .map_err(|_| SignatureParseError)?;
        Self::from_bytes(&bytes)
    }
}

impl BcsEncode for SuiSignature {
    fn bcs_encode(&self, writer: &mut BcsWriter) {
        // Inside BCS containers a signature is serialized as `bytes`, i.e.
        // ULEB128 length prefix + the raw `flag || sig || pk` serialization.
        writer.write_bytes(&self.to_bytes());
    }
}

/// Error returned when parsing a [`SuiSignature`] or [`SignatureScheme`]
/// fails.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SignatureParseError;

impl fmt::Display for SignatureParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "invalid Sui signature: expected flag 0x00/0x01/0x02 followed by \
             a 64-byte signature and the scheme's public key"
        )
    }
}

impl std::error::Error for SignatureParseError {}

// The JSON form is the base64 `flag || sig || pk` string used by the Sui
// JSON-RPC API. A derive would emit the 64-byte array field, which serde
// cannot derive and RPC would not accept, so serde is implemented by hand.
#[cfg(feature = "serde")]
impl Serialize for SuiSignature {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_base64())
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for SuiSignature {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Self::from_base64(&s).map_err(serde::de::Error::custom)
    }
}

// Schema mirrors the serde form (base64 string).
#[cfg(feature = "schemars")]
impl JsonSchema for SuiSignature {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "SuiSignature".into()
    }

    fn json_schema(generator: &mut schemars::generate::SchemaGenerator) -> schemars::Schema {
        <String>::json_schema(generator)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bcs_encoding::to_bytes;

    #[test]
    fn test_scheme_flags() {
        assert_eq!(SignatureScheme::Ed25519.flag(), 0x00);
        assert_eq!(SignatureScheme::Secp256k1.flag(), 0x01);
        assert_eq!(SignatureScheme::Secp256r1.flag(), 0x02);
        assert_eq!(
            SignatureScheme::from_flag(0x00),
            Ok(SignatureScheme::Ed25519)
        );
        assert_eq!(
            SignatureScheme::from_flag(0x01),
            Ok(SignatureScheme::Secp256k1)
        );
        assert_eq!(
            SignatureScheme::from_flag(0x02),
            Ok(SignatureScheme::Secp256r1)
        );
        assert_eq!(SignatureScheme::from_flag(0x03), Err(SignatureParseError));
    }

    /// Spec vector: the raw envelope is `flag || sig(64) || pk(32)` with no
    /// length prefix; the BCS form adds a ULEB128 length prefix.
    #[test]
    fn test_ed25519_envelope_layout() {
        let signature = SuiSignature::ed25519([0x01u8; 64], [0x02u8; 32]);

        let raw = signature.to_bytes();
        assert_eq!(raw.len(), 97);
        assert_eq!(raw[0], 0x00);
        assert_eq!(&raw[1..65], &[0x01u8; 64][..]);
        assert_eq!(&raw[65..], &[0x02u8; 32][..]);

        let bcs = to_bytes(&signature);
        assert_eq!(bcs.len(), 98);
        assert_eq!(bcs[0], 0x61); // ULEB length 97
        assert_eq!(&bcs[1..], raw.as_slice());

        assert_eq!(SuiSignature::from_bytes(&raw).unwrap(), signature);
    }

    #[test]
    fn test_secp256k1_envelope_layout() {
        let signature = SuiSignature::secp256k1([0x03u8; 64], [0x04u8; 33]);

        let raw = signature.to_bytes();
        assert_eq!(raw.len(), 98);
        assert_eq!(raw[0], 0x01);

        let bcs = to_bytes(&signature);
        assert_eq!(bcs.len(), 99);
        assert_eq!(bcs[0], 0x62); // ULEB length 98

        assert_eq!(SuiSignature::from_bytes(&raw).unwrap(), signature);
    }

    #[test]
    fn test_from_bytes_rejects_bad_input() {
        assert!(SuiSignature::from_bytes(&[]).is_err());
        assert!(SuiSignature::from_bytes(&[0x07; 97]).is_err()); // unknown flag
        assert!(SuiSignature::from_bytes(&[0x00; 96]).is_err()); // truncated
        assert!(SuiSignature::from_bytes(&[0x00; 98]).is_err()); // pk too long
        assert!(SuiSignature::from_bytes(&[0x01; 97]).is_err()); // secp needs 98
    }

    /// The raw form must match the reference SDK's serialized signature.
    #[test]
    fn test_to_bytes_against_reference_sdk() {
        let signature = SuiSignature::ed25519([0x01u8; 64], [0x02u8; 32]);
        let reference =
            sui_sdk_types::UserSignature::Simple(sui_sdk_types::SimpleSignature::Ed25519 {
                signature: sui_sdk_types::Ed25519Signature::new([0x01u8; 64]),
                public_key: sui_sdk_types::Ed25519PublicKey::new([0x02u8; 32]),
            });
        assert_eq!(signature.to_bytes(), reference.to_bytes());
        // In a BCS container the signature is length-prefixed `bytes`.
        assert_eq!(to_bytes(&signature), ::bcs::to_bytes(&reference).unwrap());
    }

    #[test]
    #[cfg(feature = "serde")]
    fn test_base64_and_serde_round_trip() {
        let signature = SuiSignature::ed25519([0x01u8; 64], [0x02u8; 32]);
        let base64 = signature.to_base64();
        let reference =
            sui_sdk_types::UserSignature::Simple(sui_sdk_types::SimpleSignature::Ed25519 {
                signature: sui_sdk_types::Ed25519Signature::new([0x01u8; 64]),
                public_key: sui_sdk_types::Ed25519PublicKey::new([0x02u8; 32]),
            });
        assert_eq!(base64, reference.to_base64());
        assert_eq!(SuiSignature::from_base64(&base64).unwrap(), signature);

        let json = serde_json::to_string(&signature).unwrap();
        assert_eq!(json, format!("\"{base64}\""));
        let back: SuiSignature = serde_json::from_str(&json).unwrap();
        assert_eq!(back, signature);
    }
}
