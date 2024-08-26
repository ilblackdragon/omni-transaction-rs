use near_sdk::serde::{Deserialize, Serialize};

pub type Address = [u8; 20];

pub type AccessList = Vec<(Address, Vec<[u8; 32]>)>;

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct Signature {
    pub v: u64,
    pub r: Vec<u8>,
    pub s: Vec<u8>,
}
