use near_crypto::PublicKey;
use near_primitives::hash::CryptoHash;
use near_primitives::transaction::Transaction;
use near_sdk::borsh;

pub struct NearTransaction {
    pub nonce: u64,
    pub sender_id: String,
    pub signer_public_key: [u8; 64],
    pub receiver_id: String,
}

impl NearTransaction {
    pub fn build_for_signing(&self) -> Vec<u8> {
        let tx = Transaction {
            signer_id: self.sender_id.parse().unwrap(),
            public_key: PublicKey::SECP256K1(self.signer_public_key.into()),
            nonce: self.nonce,
            receiver_id: self.receiver_id.parse().unwrap(),
            block_hash: CryptoHash([0; 32]),
            actions: vec![],
        };
        borsh::to_vec(&tx).expect("failed to serialize NEAR transaction")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hex;

    #[test]
    fn test_build_for_signing_for_near() {
        let tx = NearTransaction {
            nonce: 0,
            sender_id: "alice.near".to_string(),
            signer_public_key: [0u8; 64],
            receiver_id: "alice.near".to_string(),
        };
        let tx_encoded = tx.build_for_signing();
        assert_eq!(hex::encode(tx_encoded), "0a000000616c6963652e6e656172010000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000a000000616c6963652e6e656172000000000000000000000000000000000000000000000000000000000000000000000000");
    }
}
