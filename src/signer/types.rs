use near_sdk::ext_contract;
use near_sdk::serde::{Deserialize, Serialize};
use schemars::JsonSchema;

// Breaking change here, possible to keep backwards compatibility?
// Need to look at the old MPC contract to see what it is was returning and how the MPC migrated to the new format
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[serde(crate = "near_sdk::serde", tag = "scheme")]
pub enum SignatureResponse {
    Secp256k1(Secp256k1Signature),
    Ed25519 { signature: Ed25519Signature },
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[serde(crate = "near_sdk::serde")]
pub struct Secp256k1Signature {
    pub big_r: SerializableAffinePoint,
    pub s: SerializableScalar,
    pub recovery_id: u8,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[serde(crate = "near_sdk::serde")]
pub struct Ed25519Signature(
    pub Vec<u8>,
);

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[serde(crate = "near_sdk::serde")]
pub struct SerializableAffinePoint {
    pub affine_point: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[serde(crate = "near_sdk::serde")]
pub struct SerializableScalar {
    pub scalar: String,
}

// Legacy SignRequest
#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(crate = "near_sdk::serde")]
pub struct SignRequest {
    pub payload: [u8; 32],
    pub path: String,
    pub key_version: u32,
}

/// A signature payload; the right payload must be passed in for the curve.
/// The json encoding for this payload converts the bytes to hex string.
#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(crate = "near_sdk::serde")]
pub enum Payload {
    Ecdsa(String), // Hex-encoded 32 bytes
    Eddsa(String), // Hex-encoded 32-1232 bytes
}

impl Payload {
    pub fn from_legacy_ecdsa(bytes: [u8; 32]) -> Self {
        Payload::Ecdsa(hex::encode(bytes))
    }

    pub fn as_ecdsa(&self) -> Option<&str> {
        match self {
            Payload::Ecdsa(hex) => Some(hex),
            _ => None,
        }
    }

    pub fn as_eddsa(&self) -> Option<&str> {
        match self {
            Payload::Eddsa(hex) => Some(hex),
            _ => None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(crate = "near_sdk::serde")]
pub struct SignRequestArgs {
    pub path: String,
    pub payload_v2: Payload,
    pub domain_id: u64,
}

// Validation function for SignRequestArgs
impl SignRequestArgs {
    pub fn validate_payload_length(&self) -> Result<(), String> {
        match &self.payload_v2 {
            Payload::Ecdsa(hex) => {
                if hex.len() != 64 { 
                    return Err("Ecdsa payload must be 64 hex characters (32 bytes)".to_string());
                }
                Ok(())
            },
            Payload::Eddsa(hex) => {
                if hex.len() < 64 || hex.len() > 2464 { 
                    return Err("Eddsa payload must be between 64 and 2464 hex characters (32-1232 bytes)".to_string());
                }
                Ok(())
            }
        }
    }
}

// Conversion from legacy SignRequest to modern SignRequestArgs
// Is there a way to implement this so you don't have to use .into() so its fully backwards compatible?
// sign should take signRequest and signRequestArgs
impl From<SignRequest> for SignRequestArgs {
    fn from(legacy: SignRequest) -> Self {
        SignRequestArgs {
            path: legacy.path,
            payload_v2: Payload::from_legacy_ecdsa(legacy.payload),
            domain_id: legacy.key_version as u64,
        }
    }
}

#[allow(dead_code)]
#[ext_contract(mpc_contract)]
pub trait MPCContract {
    fn sign(&self, request: SignRequestArgs);
}
