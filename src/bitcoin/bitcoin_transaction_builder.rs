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
}
