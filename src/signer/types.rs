#[cfg(feature = "schemars")]
use schemars::JsonSchema;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Response containing a signature from the MPC contract
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", serde(tag = "scheme"))]
pub enum SignatureResponse {
    /// SECP256K1 signature
    Secp256k1(Secp256k1Signature),
    /// Ed25519 signature
    Ed25519 { signature: Ed25519Signature },
}

/// SECP256K1 signature components
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct Secp256k1Signature {
    pub big_r: SerializableAffinePoint,
    pub s: SerializableScalar,
    pub recovery_id: u8,
}

/// Ed25519 signature
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct Ed25519Signature(pub Vec<u8>);

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct SerializableAffinePoint {
    pub affine_point: String,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct SerializableScalar {
    pub scalar: String,
}

/// Legacy SignRequest (for backward compatibility)
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct SignRequest {
    pub payload: [u8; 32],
    pub path: String,
    pub key_version: u32,
}

/// A signature payload; the right payload must be passed in for the curve.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub enum Payload {
    /// ECDSA payload - hex-encoded 32 bytes
    Ecdsa(String),
    /// EdDSA payload - hex-encoded 32-1232 bytes
    Eddsa(String),
}

impl Payload {
    /// Create an ECDSA payload from legacy 32-byte array
    pub fn from_legacy_ecdsa(bytes: [u8; 32]) -> Self {
        Self::Ecdsa(hex::encode(bytes))
    }

    /// Get the hex string if this is an ECDSA payload
    pub fn as_ecdsa(&self) -> Option<&str> {
        match self {
            Self::Ecdsa(hex) => Some(hex),
            _ => None,
        }
    }

    /// Get the hex string if this is an EdDSA payload
    pub fn as_eddsa(&self) -> Option<&str> {
        match self {
            Self::Eddsa(hex) => Some(hex),
            _ => None,
        }
    }
}

/// Modern sign request arguments supporting both ECDSA and EdDSA
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct SignRequestArgs {
    pub path: String,
    pub payload_v2: Payload,
    pub domain_id: u64,
}

impl SignRequestArgs {
    /// Validate that the payload has the correct length for its type
    pub fn validate_payload_length(&self) -> Result<(), String> {
        match &self.payload_v2 {
            Payload::Ecdsa(hex) => {
                if hex.len() != 64 {
                    return Err("Ecdsa payload must be 64 hex characters (32 bytes)".to_string());
                }
                Ok(())
            }
            Payload::Eddsa(hex) => {
                if hex.len() < 64 || hex.len() > 2464 {
                    return Err(
                        "Eddsa payload must be between 64 and 2464 hex characters (32-1232 bytes)"
                            .to_string(),
                    );
                }
                Ok(())
            }
        }
    }
}

/// Conversion from legacy SignRequest to modern SignRequestArgs
impl From<SignRequest> for SignRequestArgs {
    fn from(legacy: SignRequest) -> Self {
        Self {
            path: legacy.path,
            payload_v2: Payload::from_legacy_ecdsa(legacy.payload),
            domain_id: legacy.key_version as u64,
        }
    }
}
