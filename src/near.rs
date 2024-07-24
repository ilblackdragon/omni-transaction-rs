
use borsh;

use near_primitives::account::id::AccountId;
use near_primitives::transaction::Transaction;
use near_crypto::PublicKey;
use near_primitives::hash::CryptoHash;

pub fn near_transaction(signer_id: String, public_key: [u8; 64], nonce: u64, receiver_id: String) -> Vec<u8> {
    let actions = vec![];
    let tx = Transaction {
        signer_id: AccountId::new_unvalidated(signer_id),
        public_key: PublicKey::SECP256K1(public_key.into()),
        nonce,
        receiver_id: AccountId::new_unvalidated(receiver_id),
        block_hash: CryptoHash([0; 32]),
        actions
    };
    borsh::to_vec(&tx).expect("failed to serialize NEAR transaction")
}