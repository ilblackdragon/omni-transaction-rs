//! Transaction builder for Aptos transactions
use crate::transaction_builder::TxBuilder;

use super::{
    aptos_transaction::AptosTransaction,
    types::{AccountAddress, TransactionPayload},
};

/// A builder for [`AptosTransaction`].
///
/// Mandatory fields (the build panics without them): `sender`, `payload`,
/// `max_gas_amount`, `gas_unit_price`, `expiration_timestamp_secs` and
/// `chain_id` — the chain id is part of the signed bytes (replay
/// protection: mainnet = 1, testnet = 2) and is never defaulted silently.
/// `sequence_number` defaults to 0 (a fresh account's first transaction).
pub struct AptosTransactionBuilder {
    sender: Option<AccountAddress>,
    sequence_number: Option<u64>,
    payload: Option<TransactionPayload>,
    max_gas_amount: Option<u64>,
    gas_unit_price: Option<u64>,
    expiration_timestamp_secs: Option<u64>,
    chain_id: Option<u8>,
}

impl Default for AptosTransactionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl TxBuilder<AptosTransaction> for AptosTransactionBuilder {
    /// Builds the [`AptosTransaction`].
    ///
    /// # Panics
    ///
    /// Panics if a mandatory field (`sender`, `payload`, `max_gas_amount`,
    /// `gas_unit_price`, `expiration_timestamp_secs` or `chain_id`) was not
    /// set.
    fn build(&self) -> AptosTransaction {
        AptosTransaction {
            sender: self.sender.expect("sender is mandatory"),
            sequence_number: self.sequence_number.unwrap_or_default(),
            payload: self.payload.clone().expect("payload is mandatory"),
            max_gas_amount: self.max_gas_amount.expect("max_gas_amount is mandatory"),
            gas_unit_price: self.gas_unit_price.expect("gas_unit_price is mandatory"),
            expiration_timestamp_secs: self
                .expiration_timestamp_secs
                .expect("expiration_timestamp_secs is mandatory"),
            chain_id: self.chain_id.expect("chain_id is mandatory"),
        }
    }
}

impl AptosTransactionBuilder {
    /// Creates a builder with no fields set.
    pub const fn new() -> Self {
        Self {
            sender: None,
            sequence_number: None,
            payload: None,
            max_gas_amount: None,
            gas_unit_price: None,
            expiration_timestamp_secs: None,
            chain_id: None,
        }
    }

    /// Sender account address of the transaction.
    pub const fn sender(mut self, sender: AccountAddress) -> Self {
        self.sender = Some(sender);
        self
    }

    /// Sequence number of the sender's account (defaults to 0).
    pub const fn sequence_number(mut self, sequence_number: u64) -> Self {
        self.sequence_number = Some(sequence_number);
        self
    }

    /// Payload the transaction executes (entry function, script or multisig).
    pub fn payload(mut self, payload: TransactionPayload) -> Self {
        self.payload = Some(payload);
        self
    }

    /// Maximal gas units to spend for this transaction.
    pub const fn max_gas_amount(mut self, max_gas_amount: u64) -> Self {
        self.max_gas_amount = Some(max_gas_amount);
        self
    }

    /// Price per gas unit, in octas.
    pub const fn gas_unit_price(mut self, gas_unit_price: u64) -> Self {
        self.gas_unit_price = Some(gas_unit_price);
        self
    }

    /// Expiration of the transaction as unix seconds; must be in the future
    /// at execution time. Inside a NEAR contract derive it from
    /// `near_sdk::env::block_timestamp_ms() / 1000` plus a margin.
    pub const fn expiration_timestamp_secs(mut self, expiration_timestamp_secs: u64) -> Self {
        self.expiration_timestamp_secs = Some(expiration_timestamp_secs);
        self
    }

    /// Chain id of the target network (mainnet = 1, testnet = 2).
    pub const fn chain_id(mut self, chain_id: u8) -> Self {
        self.chain_id = Some(chain_id);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aptos::types::{EntryFunction, Identifier, ModuleId};
    use crate::bcs_encoding::to_bytes;

    fn transfer_payload() -> TransactionPayload {
        let receiver = AccountAddress::from_hex("0xdd").unwrap();
        TransactionPayload::EntryFunction(EntryFunction::new(
            ModuleId::new(
                AccountAddress::from_hex("0x1222").unwrap(),
                Identifier::new("aptos_coin").unwrap(),
            ),
            Identifier::new("transfer").unwrap(),
            vec![],
            vec![to_bytes(&receiver), 1u64.to_le_bytes().to_vec()],
        ))
    }

    #[test]
    fn test_builder_produces_the_official_signing_message() {
        let tx = AptosTransactionBuilder::new()
            .sender(AccountAddress::from_hex("0xa550c18").unwrap())
            .sequence_number(0)
            .payload(transfer_payload())
            .max_gas_amount(2000)
            .gas_unit_price(0)
            .expiration_timestamp_secs(u64::MAX)
            .chain_id(4)
            .build();

        assert_eq!(
            hex::encode(tx.build_for_signing()),
            "b5e97db07fa0bd0e5598aa3643a9bc6f6693bddc1a9fec9e674a461eaa00b193000000000000000000000000000000000000000000000000000000000a550c1800000000000000000200000000000000000000000000000000000000000000000000000000000012220a6170746f735f636f696e087472616e7366657200022000000000000000000000000000000000000000000000000000000000000000dd080100000000000000d0070000000000000000000000000000ffffffffffffffff04"
        );
    }

    #[test]
    fn test_builder_matches_struct_literal_and_defaults_sequence_number() {
        let built = AptosTransactionBuilder::new()
            .sender(AccountAddress::ONE)
            .payload(transfer_payload())
            .max_gas_amount(2000)
            .gas_unit_price(100)
            .expiration_timestamp_secs(1_735_689_600)
            .chain_id(1)
            .build();

        let expected = AptosTransaction {
            sender: AccountAddress::ONE,
            sequence_number: 0,
            payload: transfer_payload(),
            max_gas_amount: 2000,
            gas_unit_price: 100,
            expiration_timestamp_secs: 1_735_689_600,
            chain_id: 1,
        };
        assert_eq!(built, expected);
    }

    #[test]
    #[should_panic(expected = "chain_id is mandatory")]
    fn test_builder_panics_without_chain_id() {
        let _ = AptosTransactionBuilder::new()
            .sender(AccountAddress::ONE)
            .payload(transfer_payload())
            .max_gas_amount(2000)
            .gas_unit_price(100)
            .expiration_timestamp_secs(1_735_689_600)
            .build();
    }
}
