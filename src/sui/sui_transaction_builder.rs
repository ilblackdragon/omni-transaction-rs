//! Transaction builder for Sui transactions
use super::sui_transaction::SuiTransaction;
use super::types::{
    CallArg, Command, GasData, ObjectRef, ProgrammableTransaction, SuiAddress,
    TransactionExpiration, TransactionKind,
};
use crate::transaction_builder::TxBuilder;

/// A builder for [`SuiTransaction`].
///
/// Mandatory fields: [`Self::sender`], the programmable transaction block
/// ([`Self::programmable`] or [`Self::kind`]), [`Self::gas_payment`] (at
/// least one gas coin), [`Self::gas_price`] and [`Self::gas_budget`]. The
/// gas owner defaults to the sender (set [`Self::gas_owner`] for sponsored
/// transactions) and the expiration to [`TransactionExpiration::None`].
///
/// ###### Example:
///
/// ```rust
/// use omni_transaction::sui::types::{Argument, CallArg, Command, ObjectDigest, ObjectRef};
/// use omni_transaction::sui::utils::parse_sui_address;
/// use omni_transaction::{TransactionBuilder, TxBuilder, SUI};
///
/// let sui_tx = TransactionBuilder::new::<SUI>()
///     .sender(parse_sui_address("0x2"))
///     .programmable(
///         vec![
///             CallArg::pure_u64(1_000_000),
///             CallArg::pure_address(parse_sui_address("0x3")),
///         ],
///         vec![
///             Command::SplitCoins {
///                 coin: Argument::GasCoin,
///                 amounts: vec![Argument::Input(0)],
///             },
///             Command::TransferObjects {
///                 objects: vec![Argument::Result(0)],
///                 address: Argument::Input(1),
///             },
///         ],
///     )
///     .gas_payment(vec![ObjectRef::new(
///         parse_sui_address("0x1"),
///         2,
///         ObjectDigest::new([0x63u8; 32]),
///     )])
///     .gas_price(1000)
///     .gas_budget(5_000_000)
///     .build();
///
/// let digest = sui_tx.build_for_signing(); // 32-byte blake2b-256 digest
/// # assert_eq!(digest.len(), 32);
/// ```
#[derive(Debug, Clone, Default)]
pub struct SuiTransactionBuilder {
    kind: Option<TransactionKind>,
    sender: Option<SuiAddress>,
    gas_payment: Option<Vec<ObjectRef>>,
    gas_owner: Option<SuiAddress>,
    gas_price: Option<u64>,
    gas_budget: Option<u64>,
    expiration: Option<TransactionExpiration>,
}

impl SuiTransactionBuilder {
    /// Creates a new, empty builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Address sending (and, unless a gas owner is set, paying for) the
    /// transaction. Must match the signing key.
    pub const fn sender(mut self, sender: SuiAddress) -> Self {
        self.sender = Some(sender);
        self
    }

    /// The programmable transaction block: `inputs` referenced by index from
    /// `commands`, executed in order.
    pub fn programmable(mut self, inputs: Vec<CallArg>, commands: Vec<Command>) -> Self {
        self.kind = Some(TransactionKind::ProgrammableTransaction(
            ProgrammableTransaction { inputs, commands },
        ));
        self
    }

    /// The transaction kind. Equivalent to [`Self::programmable`] for the
    /// only buildable kind.
    pub fn kind(mut self, kind: TransactionKind) -> Self {
        self.kind = Some(kind);
        self
    }

    /// The coin objects paying for gas. Mandatory and non-empty: every Sui
    /// transaction must reference at least one `Coin<SUI>` gas object, and
    /// validators reject one with an empty gas payment. Fetch fresh
    /// `(id, version, digest)` references from RPC: they become stale if the
    /// objects are mutated.
    pub fn gas_payment(mut self, gas_payment: Vec<ObjectRef>) -> Self {
        self.gas_payment = Some(gas_payment);
        self
    }

    /// Owner of the gas payment when it is not the sender (sponsored
    /// transactions).
    pub const fn gas_owner(mut self, gas_owner: SuiAddress) -> Self {
        self.gas_owner = Some(gas_owner);
        self
    }

    /// Gas price in MIST per gas unit (at least the reference gas price).
    pub const fn gas_price(mut self, gas_price: u64) -> Self {
        self.gas_price = Some(gas_price);
        self
    }

    /// Maximum gas budget in MIST.
    pub const fn gas_budget(mut self, gas_budget: u64) -> Self {
        self.gas_budget = Some(gas_budget);
        self
    }

    /// Epoch-based expiration (defaults to no expiration).
    pub const fn expiration(mut self, expiration: TransactionExpiration) -> Self {
        self.expiration = Some(expiration);
        self
    }
}

impl TxBuilder<SuiTransaction> for SuiTransactionBuilder {
    /// Builds the [`SuiTransaction`].
    ///
    /// # Panics
    ///
    /// Panics if a mandatory field (sender, programmable transaction,
    /// gas payment, gas price or gas budget) is missing, or if the gas
    /// payment is an empty list: a Sui transaction must reference at least
    /// one `Coin<SUI>` gas object, so an empty payment would only be
    /// rejected by the validators after an MPC signature was paid for.
    fn build(&self) -> SuiTransaction {
        let sender = self.sender.expect("sender is mandatory");
        let kind = self
            .kind
            .clone()
            .expect("programmable transaction is mandatory");
        let payment = self.gas_payment.clone().expect("gas_payment is mandatory");
        assert!(
            !payment.is_empty(),
            "gas_payment must contain at least one gas coin"
        );
        SuiTransaction {
            kind,
            sender,
            gas_data: GasData {
                payment,
                owner: self.gas_owner.unwrap_or(sender),
                price: self.gas_price.expect("gas_price is mandatory"),
                budget: self.gas_budget.expect("gas_budget is mandatory"),
            },
            expiration: self.expiration.unwrap_or(TransactionExpiration::None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sui::types::{Argument, ObjectDigest};
    use crate::sui::utils::parse_sui_address;

    /// The single gas coin used by the golden vector.
    fn gas_coin() -> Vec<ObjectRef> {
        vec![ObjectRef::new(
            parse_sui_address("0x1"),
            2,
            ObjectDigest::new([0x63u8; 32]),
        )]
    }

    fn build_v1() -> SuiTransaction {
        SuiTransactionBuilder::new()
            .sender(parse_sui_address("0x2"))
            .programmable(
                vec![
                    CallArg::pure_u64(1_000_000),
                    CallArg::pure_address(parse_sui_address("0x3")),
                ],
                vec![
                    Command::SplitCoins {
                        coin: Argument::GasCoin,
                        amounts: vec![Argument::Input(0)],
                    },
                    Command::TransferObjects {
                        objects: vec![Argument::Result(0)],
                        address: Argument::Input(1),
                    },
                ],
            )
            .gas_payment(gas_coin())
            .gas_price(1000)
            .gas_budget(5_000_000)
            .build()
    }

    /// The builder must reproduce spec golden vector V1 byte-for-byte.
    #[test]
    fn test_builder_reproduces_v1_golden_vector() {
        let sui_tx = build_v1();
        assert_eq!(
            hex::encode(sui_tx.build_for_signing()),
            "56bb898ff33187d573e7cc2a0124fe8940c94b74d1dedb5020548eb5b4c87c31"
        );
    }

    #[test]
    fn test_gas_owner_defaults_to_sender() {
        let sui_tx = build_v1();
        assert_eq!(sui_tx.gas_data.owner, sui_tx.sender);
        assert_eq!(sui_tx.expiration, TransactionExpiration::None);

        let sponsor = parse_sui_address("0x7");
        let sponsored = SuiTransactionBuilder::new()
            .sender(parse_sui_address("0x2"))
            .programmable(vec![], vec![])
            .gas_payment(gas_coin())
            .gas_owner(sponsor)
            .gas_price(1000)
            .gas_budget(5_000_000)
            .build();
        assert_eq!(sponsored.gas_data.owner, sponsor);
    }

    #[test]
    #[should_panic(expected = "sender is mandatory")]
    fn test_build_panics_without_sender() {
        let _ = SuiTransactionBuilder::new()
            .programmable(vec![], vec![])
            .gas_payment(gas_coin())
            .gas_price(1000)
            .gas_budget(5_000_000)
            .build();
    }

    #[test]
    #[should_panic(expected = "programmable transaction is mandatory")]
    fn test_build_panics_without_kind() {
        let _ = SuiTransactionBuilder::new()
            .sender(parse_sui_address("0x2"))
            .gas_payment(gas_coin())
            .gas_price(1000)
            .gas_budget(5_000_000)
            .build();
    }

    /// A transaction must reference at least one `Coin<SUI>` gas object, so an
    /// unset gas payment must fail here and not at the validators.
    #[test]
    #[should_panic(expected = "gas_payment is mandatory")]
    fn test_build_panics_without_gas_payment() {
        let _ = SuiTransactionBuilder::new()
            .sender(parse_sui_address("0x2"))
            .programmable(vec![], vec![])
            .gas_price(1000)
            .gas_budget(5_000_000)
            .build();
    }

    /// An explicitly empty gas payment is just as unusable as an unset one.
    #[test]
    #[should_panic(expected = "gas_payment must contain at least one gas coin")]
    fn test_build_panics_with_empty_gas_payment() {
        let _ = SuiTransactionBuilder::new()
            .sender(parse_sui_address("0x2"))
            .programmable(vec![], vec![])
            .gas_payment(vec![])
            .gas_price(1000)
            .gas_budget(5_000_000)
            .build();
    }

    #[test]
    #[should_panic(expected = "gas_price is mandatory")]
    fn test_build_panics_without_gas_price() {
        let _ = SuiTransactionBuilder::new()
            .sender(parse_sui_address("0x2"))
            .programmable(vec![], vec![])
            .gas_payment(gas_coin())
            .gas_budget(5_000_000)
            .build();
    }

    #[test]
    #[should_panic(expected = "gas_budget is mandatory")]
    fn test_build_panics_without_gas_budget() {
        let _ = SuiTransactionBuilder::new()
            .sender(parse_sui_address("0x2"))
            .programmable(vec![], vec![])
            .gas_payment(gas_coin())
            .gas_price(1000)
            .build();
    }
}
