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

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(crate = "near_sdk::serde")]
pub enum Payload {
    Ecdsa([u8; 32]),
    Ed25519(Vec<u8>), // For Ed25519, one should use the constructor to enforce correct payload length
}

// Constructors for Payload to enforce correct payload length for Ed25519
impl Payload {
    pub fn new_ecdsa(payload: [u8; 32]) -> Self {
        Payload::Ecdsa(payload)
    }
    
    pub fn new_ed25519(payload: Vec<u8>) -> Result<Self, String> {
        if payload.len() < 32 {
            return Err("Ed25519 payload must be at least 32 bytes".to_string());
        }
        if payload.len() > 1232 {
            return Err("Ed25519 payload must be at most 1232 bytes".to_string());
        }
        Ok(Payload::Ed25519(payload))
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
    pub fn validate(&self) -> Result<(), String> {
        match &self.payload_v2 {
            Payload::Ecdsa(_) => Ok(()),
            Payload::Ed25519(data) => {
                if data.len() < 32 {
                    Err("Ed25519 payload must be at least 32 bytes".to_string())
                } else if data.len() > 1232 {
                    Err("Ed25519 payload must be at most 1232 bytes".to_string())
                } else {
                    Ok(())
                }
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
            payload_v2: Payload::Ecdsa(legacy.payload),
            domain_id: legacy.key_version as u64,
        }
    }
}

#[allow(dead_code)]
#[ext_contract(mpc_contract)]
pub trait MPCContract {
    fn sign(&self, request: SignRequestArgs);
}
