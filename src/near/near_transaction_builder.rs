use super::near_transaction::NearTransaction;
use crate::transaction_builder::TxBuilder;

pub struct NearTransactionBuilder {
    pub nonce: Option<u64>,
    pub sender_id: Option<String>,
    pub signer_public_key: Option<[u8; 64]>,
    pub receiver_id: Option<String>,
}

impl Default for NearTransactionBuilder {
    fn default() -> Self {
        NearTransactionBuilder::new()
    }
}

impl TxBuilder<NearTransaction> for NearTransactionBuilder {
    fn build(&self) -> NearTransaction {
        NearTransaction {
            nonce: self.nonce.unwrap_or_default(),
            sender_id: self.sender_id.clone().expect("Missing sender ID"),
            signer_public_key: self.signer_public_key.expect("Missing signer public key"),
            receiver_id: self.receiver_id.clone().unwrap_or_default(),
        }
    }
}

impl NearTransactionBuilder {
    pub fn new() -> Self {
        Self {
            nonce: None,
            sender_id: None,
            signer_public_key: None,
            receiver_id: None,
        }
    }

    pub fn nonce(mut self, nonce: u64) -> Self {
        self.nonce = Some(nonce);
        self
    }

    pub fn sender_id(mut self, sender_id: String) -> Self {
        self.sender_id = Some(sender_id);
        self
    }

    pub fn signer_public_key(mut self, signer_public_key: [u8; 64]) -> Self {
        self.signer_public_key = Some(signer_public_key);
        self
    }

    pub fn receiver_id(mut self, receiver_id: String) -> Self {
        self.receiver_id = Some(receiver_id);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hex;

    #[test]
    fn test_near_transaction_builder() {
        let near_transaction = NearTransactionBuilder::new()
            .nonce(0)
            .sender_id("alice.near".to_string())
            .signer_public_key([0u8; 64])
            .receiver_id("alice.near".to_string())
            .build();

        let tx_encoded = near_transaction.build_for_signing();

        assert_eq!(hex::encode(tx_encoded), "0a000000616c6963652e6e656172010000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000a000000616c6963652e6e656172000000000000000000000000000000000000000000000000000000000000000000000000");
    }
}
