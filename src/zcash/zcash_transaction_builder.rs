//! Transaction builder for Zcash transactions
use super::constants::MAX_EXPIRY_HEIGHT;
use super::types::{ConsensusBranchId, TxIn, TxOut};
use super::zcash_transaction::ZcashTransaction;
use crate::transaction_builder::TxBuilder;

/// Builder for transparent-only Zcash v5 transactions.
///
/// Mandatory fields: [`consensus_branch_id`](Self::consensus_branch_id),
/// [`inputs`](Self::inputs) and [`outputs`](Self::outputs). Optional fields
/// default to `0`: [`lock_time`](Self::lock_time) (no lock time) and
/// [`expiry_height`](Self::expiry_height) (never expires; `zcashd` uses tip
/// height + 40 by default, see [ZIP-203]).
///
/// [ZIP-203]: https://zips.z.cash/zip-0203
pub struct ZcashTransactionBuilder {
    consensus_branch_id: Option<ConsensusBranchId>,
    lock_time: Option<u32>,
    expiry_height: Option<u32>,
    inputs: Option<Vec<TxIn>>,
    outputs: Option<Vec<TxOut>>,
}

impl Default for ZcashTransactionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl TxBuilder<ZcashTransaction> for ZcashTransactionBuilder {
    /// Builds the [`ZcashTransaction`].
    ///
    /// # Panics
    ///
    /// * If `consensus_branch_id`, `inputs` or `outputs` is missing.
    /// * If `expiry_height` exceeds 499,999,999 ([ZIP-203]).
    ///
    /// [ZIP-203]: https://zips.z.cash/zip-0203
    fn build(&self) -> ZcashTransaction {
        let expiry_height = self.expiry_height.unwrap_or_default();
        assert!(
            expiry_height <= MAX_EXPIRY_HEIGHT,
            "expiry_height must be at most 499,999,999 (ZIP-203)"
        );
        ZcashTransaction {
            consensus_branch_id: self
                .consensus_branch_id
                .expect("consensus_branch_id is mandatory"),
            lock_time: self.lock_time.unwrap_or_default(),
            expiry_height,
            input: self.inputs.clone().expect("Missing inputs"),
            output: self.outputs.clone().expect("Missing outputs"),
        }
    }
}

impl ZcashTransactionBuilder {
    /// Creates a new builder with no fields set.
    pub const fn new() -> Self {
        Self {
            consensus_branch_id: None,
            lock_time: None,
            expiry_height: None,
            inputs: None,
            outputs: None,
        }
    }

    /// Consensus branch id of the epoch the transaction will be mined in.
    ///
    /// Mandatory; it rotates with every network upgrade, so re-check the
    /// active branch id at broadcast time.
    pub const fn consensus_branch_id(mut self, consensus_branch_id: ConsensusBranchId) -> Self {
        self.consensus_branch_id = Some(consensus_branch_id);
        self
    }

    /// Lock time of the transaction, as in Bitcoin. Defaults to `0`.
    pub const fn lock_time(mut self, lock_time: u32) -> Self {
        self.lock_time = Some(lock_time);
        self
    }

    /// Expiry height of the transaction ([ZIP-203]). Defaults to `0` (never
    /// expires); must be at most 499,999,999.
    ///
    /// [ZIP-203]: https://zips.z.cash/zip-0203
    pub const fn expiry_height(mut self, expiry_height: u32) -> Self {
        self.expiry_height = Some(expiry_height);
        self
    }

    /// Transparent inputs of the transaction.
    pub fn inputs(mut self, inputs: Vec<TxIn>) -> Self {
        self.inputs = Some(inputs);
        self
    }

    /// Transparent outputs of the transaction.
    pub fn outputs(mut self, outputs: Vec<TxOut>) -> Self {
        self.outputs = Some(outputs);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::zcash::types::{Amount, ScriptBuf, Sequence, TxIn, TxOut, Witness};

    fn sample_input() -> TxIn {
        TxIn {
            previous_output: crate::zcash::types::OutPoint::default(),
            script_sig: ScriptBuf::default(),
            sequence: Sequence::MAX,
            witness: Witness::default(),
        }
    }

    fn sample_output() -> TxOut {
        TxOut {
            value: Amount::from_sat(5_000),
            script_pubkey: ScriptBuf::default(),
        }
    }

    #[test]
    fn test_build() {
        let tx = ZcashTransactionBuilder::new()
            .consensus_branch_id(ConsensusBranchId::Nu6_3)
            .lock_time(1)
            .expiry_height(2_500_000)
            .inputs(vec![sample_input()])
            .outputs(vec![sample_output()])
            .build();

        assert_eq!(tx.consensus_branch_id, ConsensusBranchId::Nu6_3);
        assert_eq!(tx.lock_time, 1);
        assert_eq!(tx.expiry_height, 2_500_000);
        assert_eq!(tx.input.len(), 1);
        assert_eq!(tx.output.len(), 1);
    }

    #[test]
    fn test_build_with_defaults() {
        let tx = ZcashTransactionBuilder::new()
            .consensus_branch_id(ConsensusBranchId::Custom(0xDEAD_BEEF))
            .inputs(vec![])
            .outputs(vec![])
            .build();

        assert_eq!(tx.lock_time, 0);
        assert_eq!(tx.expiry_height, 0);
        assert_eq!(tx.consensus_branch_id.to_u32(), 0xDEAD_BEEF);
    }

    #[test]
    #[should_panic(expected = "consensus_branch_id is mandatory")]
    fn test_build_panics_without_consensus_branch_id() {
        ZcashTransactionBuilder::new()
            .inputs(vec![])
            .outputs(vec![])
            .build();
    }

    #[test]
    #[should_panic(expected = "expiry_height must be at most 499,999,999")]
    fn test_build_panics_on_expiry_height_above_maximum() {
        ZcashTransactionBuilder::new()
            .consensus_branch_id(ConsensusBranchId::Nu6_3)
            .expiry_height(500_000_000)
            .inputs(vec![])
            .outputs(vec![])
            .build();
    }
}
