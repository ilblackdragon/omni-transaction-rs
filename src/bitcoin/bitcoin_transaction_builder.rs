use super::{
    bitcoin_transaction::BitcoinTransaction,
    types::{LockTime, TxIn, TxOut, Version},
};
use crate::transaction_builder::TxBuilder;

pub struct BitcoinTransactionBuilder {
    pub version: Option<Version>,
    pub lock_time: Option<LockTime>,
    pub inputs: Option<Vec<TxIn>>,
    pub outputs: Option<Vec<TxOut>>,
}

impl Default for BitcoinTransactionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl TxBuilder<BitcoinTransaction> for BitcoinTransactionBuilder {
    fn build(&self) -> BitcoinTransaction {
        BitcoinTransaction {
            version: self.version.expect("Missing version"),
            lock_time: self.lock_time.expect("Missing lock time"),
            input: self.inputs.clone().expect("Missing inputs"),
            output: self.outputs.clone().expect("Missing outputs"),
        }
    }
}

impl BitcoinTransactionBuilder {
    pub const fn new() -> Self {
        Self {
            version: None,
            lock_time: None,
            inputs: None,
            outputs: None,
        }
    }

    pub const fn version(mut self, version: Version) -> Self {
        self.version = Some(version);
        self
    }

    pub const fn lock_time(mut self, lock_time: LockTime) -> Self {
        self.lock_time = Some(lock_time);
        self
    }

    pub fn inputs(mut self, inputs: Vec<TxIn>) -> Self {
        self.inputs = Some(inputs);
        self
    }

    pub fn outputs(mut self, outputs: Vec<TxOut>) -> Self {
        self.outputs = Some(outputs);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build() {
        let block_height = 10000;
        let builder = BitcoinTransactionBuilder::new()
            .version(Version::One)
            .lock_time(LockTime::from_height(block_height).unwrap())
            .inputs(vec![])
            .outputs(vec![])
            .build();

        assert_eq!(builder.version, Version::One);
        assert_eq!(
            builder.lock_time,
            LockTime::from_height(block_height).unwrap()
        );
    }

    #[test]
    fn test_sighash() {
        let block_height = 10000;
        let _builder = BitcoinTransactionBuilder::new()
            .version(Version::One)
            .lock_time(LockTime::from_height(block_height).unwrap())
            .inputs(vec![])
            .outputs(vec![])
            .build();
    }

    // #[test]
    // fn test_sighash_all() {
    // -------------------------------------------------------------------------------------------------
    // Create an equivalent transaction using the Bitcoin library
    //     let txin_btc = TxIn {
    //         previous_output: OutPoint::new(
    //             first_unspent["txid"].as_str().unwrap().parse()?,
    //             first_unspent["vout"].as_u64().unwrap() as u32,
    //         ),
    //         script_sig: ScriptBuf::new(), // Initially empty, will be filled later with the signature
    //         sequence: Sequence::MAX,
    //         witness: Witness::default(),
    //     };

    //     let txout_btc = TxOut {
    //         value: SPEND_AMOUNT,
    //         script_pubkey: alice.script_pubkey.clone(),
    //     };

    //     let utxo_amount_btc = Amount::from_btc(first_unspent["amount"].as_f64().unwrap())?;
    //     let change_amount: Amount = utxo_amount_btc - SPEND_AMOUNT - Amount::from_sat(1000); // 1000 satoshis for fee

    //     let change_txout_btc = TxOut {
    //         value: change_amount,
    //         script_pubkey: bob.script_pubkey.clone(),
    //     };

    //     let mut tx_btc = Transaction {
    //         version: transaction::Version::ONE,
    //         lock_time: LockTime::Blocks(Height::from_consensus(1).unwrap()),
    //         input: vec![txin_btc],
    //         output: vec![txout_btc, change_txout_btc],
    //     };

    //     // Get the sighash to sign.
    //     let sighasher = SighashCache::new(&mut tx_btc);
    //     let sighash_btc = sighasher
    //         .legacy_signature_hash(0, &bob.script_pubkey, sighash_type.to_u32())
    //         .expect("failed to create sighash");

    //     println!("sighash btc: {:?}", sighash_btc);
    //     println!("sighash: {:?}", sighash_omni.to_byte_array());
    //     println!("sighash btc: {:?}", sighash_btc.to_byte_array());

    //     assert_eq!(
    //         sighash_omni.to_byte_array(),
    //         sighash_btc.to_byte_array(),
    //         "sighash btc is not equal to sighash"
    //     );
    // }
}
