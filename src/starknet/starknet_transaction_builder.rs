//! Transaction builder for Starknet transactions.
//!
//! ###### Example:
//!
//! ```rust
//! use omni_transaction::starknet::types::{Call, Felt, ResourceBounds};
//! use omni_transaction::starknet::utils::{get_selector_from_name, CHAIN_ID_SEPOLIA};
//! use omni_transaction::{TransactionBuilder, TxBuilder, STARKNET};
//!
//! let calls = vec![Call {
//!     to: Felt::from_hex("0x4138fd51f90d171df37e9d4419c8cdb67d525840c58f8a5c347be93a1c5277d")
//!         .unwrap(),
//!     selector: get_selector_from_name("transfer"),
//!     calldata: vec![Felt::from(1u64), Felt::from(2u64)],
//! }];
//!
//! let tx = TransactionBuilder::new::<STARKNET>()
//!     .chain_id(CHAIN_ID_SEPOLIA)
//!     .sender_address(
//!         Felt::from_hex("0x745d525a3582e91299d8d7c71730ffc4b1f191f5b219d800334bc0edad0983b")
//!             .unwrap(),
//!     )
//!     .nonce(Felt::from(1u64))
//!     .calls(&calls)
//!     .l1_gas(ResourceBounds::new(0x186a0, 0x2d79883d20000))
//!     .l2_gas(ResourceBounds::new(0x5f5e100, 0xba43b7400))
//!     .l1_data_gas(ResourceBounds::new(0x186a0, 0x2d79883d20000))
//!     .build();
//!
//! // The payload to sign is the 32-byte big-endian transaction-hash felt.
//! let payload = tx.build_for_signing();
//! assert_eq!(payload.len(), 32);
//! ```
use crate::transaction_builder::TxBuilder;

use super::{
    starknet_transaction::StarknetTransaction,
    types::{Call, DataAvailabilityMode, Felt, ResourceBounds},
    utils::encode_calls,
};

/// A builder for Starknet INVOKE v3 transactions.
///
/// `chain_id`, `sender_address` and `nonce` are mandatory; [`TxBuilder::build`] panics
/// when any of them is missing. Every other field has a sensible default: empty
/// calldata, zero tip, zero resource bounds, empty paymaster / account-deployment data,
/// and `L1` data-availability modes.
pub struct StarknetTransactionBuilder {
    chain_id: Option<Felt>,
    sender_address: Option<Felt>,
    nonce: Option<Felt>,
    calldata: Option<Vec<Felt>>,
    tip: Option<u64>,
    l1_gas: Option<ResourceBounds>,
    l2_gas: Option<ResourceBounds>,
    l1_data_gas: Option<ResourceBounds>,
    paymaster_data: Option<Vec<Felt>>,
    account_deployment_data: Option<Vec<Felt>>,
    nonce_data_availability_mode: Option<DataAvailabilityMode>,
    fee_data_availability_mode: Option<DataAvailabilityMode>,
}

impl Default for StarknetTransactionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl TxBuilder<StarknetTransaction> for StarknetTransactionBuilder {
    /// Builds the [`StarknetTransaction`].
    ///
    /// # Panics
    ///
    /// Panics if `chain_id`, `sender_address` or `nonce` is missing.
    fn build(&self) -> StarknetTransaction {
        StarknetTransaction {
            chain_id: self.chain_id.expect("chain_id is mandatory"),
            sender_address: self.sender_address.expect("sender_address is mandatory"),
            nonce: self.nonce.expect("nonce is mandatory"),
            calldata: self.calldata.clone().unwrap_or_default(),
            tip: self.tip.unwrap_or_default(),
            l1_gas: self.l1_gas.unwrap_or_default(),
            l2_gas: self.l2_gas.unwrap_or_default(),
            l1_data_gas: self.l1_data_gas.unwrap_or_default(),
            paymaster_data: self.paymaster_data.clone().unwrap_or_default(),
            account_deployment_data: self.account_deployment_data.clone().unwrap_or_default(),
            nonce_data_availability_mode: self.nonce_data_availability_mode.unwrap_or_default(),
            fee_data_availability_mode: self.fee_data_availability_mode.unwrap_or_default(),
        }
    }
}

impl StarknetTransactionBuilder {
    /// Creates a new builder with no fields set.
    pub const fn new() -> Self {
        Self {
            chain_id: None,
            sender_address: None,
            nonce: None,
            calldata: None,
            tip: None,
            l1_gas: None,
            l2_gas: None,
            l1_data_gas: None,
            paymaster_data: None,
            account_deployment_data: None,
            nonce_data_availability_mode: None,
            fee_data_availability_mode: None,
        }
    }

    /// Chain id felt of the target network, e.g.
    /// [`CHAIN_ID_MAINNET`](crate::starknet::utils::CHAIN_ID_MAINNET) or
    /// [`CHAIN_ID_SEPOLIA`](crate::starknet::utils::CHAIN_ID_SEPOLIA).
    pub const fn chain_id(mut self, chain_id: Felt) -> Self {
        self.chain_id = Some(chain_id);
        self
    }

    /// Address of the sender account contract.
    pub const fn sender_address(mut self, sender_address: Felt) -> Self {
        self.sender_address = Some(sender_address);
        self
    }

    /// Account nonce of the transaction.
    pub const fn nonce(mut self, nonce: Felt) -> Self {
        self.nonce = Some(nonce);
        self
    }

    /// Already-encoded `__execute__` calldata felts
    /// (see [`encode_calls`](crate::starknet::utils::encode_calls)).
    /// Alternative to [`Self::calls`].
    pub fn calldata(mut self, calldata: Vec<Felt>) -> Self {
        self.calldata = Some(calldata);
        self
    }

    /// Contract calls of the transaction; encoded into `__execute__` calldata with
    /// [`encode_calls`](crate::starknet::utils::encode_calls) (SNIP-6 "new" encoding).
    /// Alternative to [`Self::calldata`].
    pub fn calls(mut self, calls: &[Call]) -> Self {
        self.calldata = Some(encode_calls(calls));
        self
    }

    /// Tip to the sequencer, in fri.
    pub const fn tip(mut self, tip: u64) -> Self {
        self.tip = Some(tip);
        self
    }

    /// Resource bounds for L1 gas.
    pub const fn l1_gas(mut self, l1_gas: ResourceBounds) -> Self {
        self.l1_gas = Some(l1_gas);
        self
    }

    /// Resource bounds for L2 gas.
    pub const fn l2_gas(mut self, l2_gas: ResourceBounds) -> Self {
        self.l2_gas = Some(l2_gas);
        self
    }

    /// Resource bounds for L1 data gas (blob data).
    pub const fn l1_data_gas(mut self, l1_data_gas: ResourceBounds) -> Self {
        self.l1_data_gas = Some(l1_data_gas);
        self
    }

    /// Paymaster data. Must be empty on current networks (the default).
    pub fn paymaster_data(mut self, paymaster_data: Vec<Felt>) -> Self {
        self.paymaster_data = Some(paymaster_data);
        self
    }

    /// Account deployment data. Must be empty on current networks (the default).
    pub fn account_deployment_data(mut self, account_deployment_data: Vec<Felt>) -> Self {
        self.account_deployment_data = Some(account_deployment_data);
        self
    }

    /// Data-availability mode of the nonce (defaults to `L1`).
    pub const fn nonce_data_availability_mode(mut self, mode: DataAvailabilityMode) -> Self {
        self.nonce_data_availability_mode = Some(mode);
        self
    }

    /// Data-availability mode of the fee (defaults to `L1`).
    pub const fn fee_data_availability_mode(mut self, mode: DataAvailabilityMode) -> Self {
        self.fee_data_availability_mode = Some(mode);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::starknet::utils::CHAIN_ID_SEPOLIA;
    use crate::transaction_builder::TransactionBuilder;
    use crate::transaction_builders::STARKNET;

    fn reference_calls() -> Vec<Call> {
        vec![Call {
            to: Felt::from_hex_unchecked(
                "0x4138fd51f90d171df37e9d4419c8cdb67d525840c58f8a5c347be93a1c5277d",
            ),
            selector: Felt::from_hex_unchecked(
                "0x2468d193cd15b621b24c2a602b8dbcfa5eaa14f88416c40c09d7fd12592cb4b",
            ),
            calldata: vec![],
        }]
    }

    fn reference_builder() -> StarknetTransactionBuilder {
        StarknetTransactionBuilder::new()
            .chain_id(CHAIN_ID_SEPOLIA)
            .sender_address(Felt::from_hex_unchecked(
                "0x745d525a3582e91299d8d7c71730ffc4b1f191f5b219d800334bc0edad0983b",
            ))
            .nonce(Felt::from_hex_unchecked("0x9803"))
            .calls(&reference_calls())
            .l1_gas(ResourceBounds::new(0x186a0, 0x2d79883d20000))
            .l2_gas(ResourceBounds::new(0x5f5e100, 0xba43b7400))
            .l1_data_gas(ResourceBounds::new(0x186a0, 0x2d79883d20000))
    }

    #[test]
    fn test_builder_produces_real_sepolia_transaction_hash() {
        let tx = reference_builder().build();

        assert_eq!(
            hex::encode(tx.build_for_signing()),
            "076b52e17bc09064bd986ead34263e6305ef3cecfb3ae9e19b86bf4f1a1a20ea"
        );
    }

    #[test]
    fn test_builder_defaults() {
        let tx = reference_builder().build();

        assert_eq!(tx.tip, 0);
        assert_eq!(tx.paymaster_data, Vec::<Felt>::new());
        assert_eq!(tx.account_deployment_data, Vec::<Felt>::new());
        assert_eq!(tx.nonce_data_availability_mode, DataAvailabilityMode::L1);
        assert_eq!(tx.fee_data_availability_mode, DataAvailabilityMode::L1);
    }

    #[test]
    fn test_calls_and_calldata_setters_are_equivalent() {
        let via_calls = reference_builder().build();
        let via_calldata = reference_builder()
            .calldata(encode_calls(&reference_calls()))
            .build();

        assert_eq!(via_calls, via_calldata);
    }

    #[test]
    fn test_all_setters_are_applied() {
        let tx = reference_builder()
            .tip(0x1234)
            .paymaster_data(vec![Felt::ONE])
            .account_deployment_data(vec![Felt::TWO])
            .nonce_data_availability_mode(DataAvailabilityMode::L2)
            .fee_data_availability_mode(DataAvailabilityMode::L2)
            .build();

        assert_eq!(tx.tip, 0x1234);
        assert_eq!(tx.paymaster_data, vec![Felt::ONE]);
        assert_eq!(tx.account_deployment_data, vec![Felt::TWO]);
        assert_eq!(tx.nonce_data_availability_mode, DataAvailabilityMode::L2);
        assert_eq!(tx.fee_data_availability_mode, DataAvailabilityMode::L2);
    }

    #[test]
    fn test_typed_transaction_builder_matches_direct_builder() {
        let typed = TransactionBuilder::new::<STARKNET>()
            .chain_id(CHAIN_ID_SEPOLIA)
            .sender_address(Felt::from_hex_unchecked(
                "0x745d525a3582e91299d8d7c71730ffc4b1f191f5b219d800334bc0edad0983b",
            ))
            .nonce(Felt::from_hex_unchecked("0x9803"))
            .calls(&reference_calls())
            .l1_gas(ResourceBounds::new(0x186a0, 0x2d79883d20000))
            .l2_gas(ResourceBounds::new(0x5f5e100, 0xba43b7400))
            .l1_data_gas(ResourceBounds::new(0x186a0, 0x2d79883d20000))
            .build();

        assert_eq!(typed, reference_builder().build());
    }

    #[test]
    #[should_panic(expected = "chain_id is mandatory")]
    fn test_build_panics_without_chain_id() {
        let _ = StarknetTransactionBuilder::new()
            .sender_address(Felt::ONE)
            .nonce(Felt::ZERO)
            .build();
    }

    #[test]
    #[should_panic(expected = "sender_address is mandatory")]
    fn test_build_panics_without_sender_address() {
        let _ = StarknetTransactionBuilder::new()
            .chain_id(CHAIN_ID_SEPOLIA)
            .nonce(Felt::ZERO)
            .build();
    }

    #[test]
    #[should_panic(expected = "nonce is mandatory")]
    fn test_build_panics_without_nonce() {
        let _ = StarknetTransactionBuilder::new()
            .chain_id(CHAIN_ID_SEPOLIA)
            .sender_address(Felt::ONE)
            .build();
    }
}
