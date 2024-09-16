use borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::serde::{Deserialize, Serialize};

use super::{types::LockTime, types::TxIn, types::TxOut, types::Version};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct BitcoinTransaction {
    /// The protocol version, is currently expected to be 1 or 2 (BIP 68).
    pub version: Version,
    /// Block height or timestamp. Transaction cannot be included in a block until this height/time.
    ///
    /// ### Relevant BIPs
    ///
    /// * [BIP-65 OP_CHECKLOCKTIMEVERIFY](https://github.com/bitcoin/bips/blob/master/bip-0065.mediawiki)
    /// * [BIP-113 Median time-past as endpoint for lock-time calculations](https://github.com/bitcoin/bips/blob/master/bip-0113.mediawiki)
    pub lock_time: LockTime,
    // /// List of transaction inputs.
    pub input: Vec<TxIn>,
    // /// List of transaction outputs.
    pub output: Vec<TxOut>,
}

impl BitcoinTransaction {
    pub fn build_for_signing(&self) -> Vec<u8> {
        borsh::to_vec(self).expect("failed to serialize Bitcoin transaction")
    }

    // pub fn build_with_signature(&self, signature: Signature) -> Vec<u8> {
    //     let signed_tx = SignedTransaction {
    //         transaction: self.clone(),
    //         signature,
    //     };
    //     borsh::to_vec(&signed_tx).expect("failed to serialize Bitcoin transaction")
    // }

    // pub fn from_json(json: &str) -> Result<Self, near_sdk::serde_json::Error> {
    //     near_sdk::serde_json::from_str(json)
    // }
}

// build for signing
// build with signature

#[cfg(test)]
mod tests {
    use super::*;
    use bitcoin::absolute::LockTime as BitcoinLockTime;
    use bitcoin::consensus::Encodable;
    use bitcoin::transaction::{Transaction as BitcoinTransaction, Version as BitcoinVersion};

    #[test]
    fn test_build_for_signing_for_bitcoin_against_rust_bitcoin() {
        let height = 1000000;
        let version = 2;
        let tx = BitcoinTransaction {
            version: BitcoinVersion(version),
            lock_time: BitcoinLockTime::from_height(height).unwrap(),
            input: vec![],
            output: vec![],
        };

        let mut buf = [0u8; 1024];
        let size = tx.consensus_encode(&mut &mut buf[..]).unwrap();

        println!("size: {:?}", size);
        println!("buf: {:?}", buf);
        // let mut buf = [0u8; 1024];
        // let raw_tx = hex!(SOME_TX);
        // let tx: Transaction = Decodable::consensus_decode(&mut raw_tx.as_slice()).unwrap();

        //
        // assert_eq!(size, SOME_TX.len() / 2);
        // assert_eq!(raw_tx, &buf[..size]);
        // let serialized = tx.build_for_signing();
        // println!("serialized: {:?}", serialized);

        // let tx = Transaction::from_bytes(&serialized).unwrap();
        // println!("tx: {:?}", tx);
    }
}
