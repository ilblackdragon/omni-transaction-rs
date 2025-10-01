use near_sdk::ext_contract;
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::NearToken;
use schemars::JsonSchema;

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[serde(crate = "near_sdk::serde")]
pub struct SignatureResponse {
    pub big_r: SerializableAffinePoint,
    pub s: SerializableScalar,
    pub recovery_id: u8,
}

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

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(crate = "near_sdk::serde")]
pub struct SignRequest {
    pub payload: [u8; 32],
    pub path: String,
    pub key_version: u32,
}

#[allow(dead_code)]
#[ext_contract(mpc_contract)]
pub trait MPCContract {
    fn sign(&self, request: SignRequest);
    fn experimental_signature_deposit(&self) -> NearToken;
}
