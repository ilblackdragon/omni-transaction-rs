//! Transaction builder for TON transactions.
use crate::transaction_builder::TxBuilder;

use super::{
    ton_transaction::TonTransaction,
    types::{InternalMessage, WalletVersion},
};

/// Fluent builder for [`TonTransaction`].
///
/// Mandatory fields: [`Self::public_key`], [`Self::seqno`] and
/// [`Self::valid_until`]. Everything else has sensible defaults: wallet
/// version v5r1, workchain 0, the conventional wallet id for the chosen
/// version, no deployment.
///
/// ###### Example:
///
/// ```rust
/// use omni_transaction::ton::types::{Coins, InternalMessage, TonAddress};
/// use omni_transaction::{TransactionBuilder, TxBuilder, TON};
///
/// let dest: TonAddress = "EQCD39VS5jcptHL8vMjEXrzGaRcCVYto7HUn4bpAOg8xqB2N"
///     .parse()
///     .unwrap();
///
/// let ton_tx = TransactionBuilder::new::<TON>()
///     .public_key([0x17; 32])
///     .seqno(1)
///     .valid_until(1735689600)
///     .add_message(InternalMessage::new(dest, Coins::from_nano(50_000_000)))
///     .build();
///
/// let payload = ton_tx.build_for_signing();
/// assert_eq!(payload.len(), 32);
/// ```
#[derive(Debug, Default, Clone)]
pub struct TonTransactionBuilder {
    wallet_version: WalletVersion,
    workchain: i8,
    public_key: Option<[u8; 32]>,
    wallet_id: Option<u32>,
    valid_until: Option<u64>,
    seqno: Option<u32>,
    messages: Vec<InternalMessage>,
    deploy: bool,
}

impl TxBuilder<TonTransaction> for TonTransactionBuilder {
    /// Builds the [`TonTransaction`].
    ///
    /// # Panics
    ///
    /// Panics if `public_key`, `seqno` or `valid_until` is missing, or if
    /// `valid_until` does not fit in the 32-bit unix timestamp the wire
    /// format carries.
    fn build(&self) -> TonTransaction {
        let valid_until = self.valid_until.expect("valid_until is mandatory");
        TonTransaction {
            wallet_version: self.wallet_version,
            workchain: self.workchain,
            public_key: self.public_key.expect("public_key is mandatory"),
            wallet_id: self
                .wallet_id
                .unwrap_or_else(|| self.wallet_version.default_wallet_id(self.workchain)),
            valid_until: u32::try_from(valid_until)
                .expect("valid_until must be unix seconds fitting in u32"),
            seqno: self.seqno.expect("seqno is mandatory"),
            messages: self.messages.clone(),
            deploy: self.deploy,
        }
    }
}

impl TonTransactionBuilder {
    /// Creates a builder with default values (v5r1, workchain 0).
    pub const fn new() -> Self {
        Self {
            wallet_version: WalletVersion::V5R1,
            workchain: 0,
            public_key: None,
            wallet_id: None,
            valid_until: None,
            seqno: None,
            messages: Vec::new(),
            deploy: false,
        }
    }

    /// Wallet contract version (default v5r1).
    pub const fn wallet_version(mut self, wallet_version: WalletVersion) -> Self {
        self.wallet_version = wallet_version;
        self
    }

    /// Workchain of the wallet (default 0, the basechain).
    pub const fn workchain(mut self, workchain: i8) -> Self {
        self.workchain = workchain;
        self
    }

    /// The ed25519 public key controlling the wallet (mandatory); derives
    /// the wallet address and the `StateInit` on deployment.
    pub const fn public_key(mut self, public_key: [u8; 32]) -> Self {
        self.public_key = Some(public_key);
        self
    }

    /// v4r2 `subwallet_id` or v5r1 wallet id. Defaults to the conventional
    /// value for the chosen version and workchain
    /// (see [`WalletVersion::default_wallet_id`]).
    pub const fn wallet_id(mut self, wallet_id: u32) -> Self {
        self.wallet_id = Some(wallet_id);
        self
    }

    /// Expiration as a unix timestamp in seconds (mandatory; must fit in
    /// `u32`). The library has no clock — derive this from your environment
    /// (e.g. `env::block_timestamp` in a NEAR contract).
    pub const fn valid_until(mut self, valid_until: u64) -> Self {
        self.valid_until = Some(valid_until);
        self
    }

    /// Wallet sequence number (mandatory; fetch with the wallet `seqno`
    /// get-method, 0 for a fresh wallet).
    pub const fn seqno(mut self, seqno: u32) -> Self {
        self.seqno = Some(seqno);
        self
    }

    /// Replaces the list of internal transfers.
    pub fn messages(mut self, messages: Vec<InternalMessage>) -> Self {
        self.messages = messages;
        self
    }

    /// Appends one internal transfer.
    pub fn add_message(mut self, message: InternalMessage) -> Self {
        self.messages.push(message);
        self
    }

    /// Whether to attach the wallet `StateInit` (default `false`; set on
    /// the first transaction of a wallet).
    pub const fn deploy(mut self, deploy: bool) -> Self {
        self.deploy = deploy;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ton::types::{Coins, TonAddress};
    use crate::transaction_builder::TransactionBuilder;
    use crate::transaction_builders::TON;

    fn vector_transfer() -> InternalMessage {
        let dest: TonAddress = "EQCD39VS5jcptHL8vMjEXrzGaRcCVYto7HUn4bpAOg8xqB2N"
            .parse()
            .unwrap();
        InternalMessage::new(dest, Coins::from_nano(50_000_000))
    }

    fn vector_pubkey() -> [u8; 32] {
        hex::decode("31debe55d37c722768b137131caa6087080b2e0b60b94bd785d14575cfa498bc")
            .unwrap()
            .try_into()
            .unwrap()
    }

    /// The typed high-level builder produces the spec's v5r1 signing
    /// payload with all defaults applied.
    #[test]
    fn test_builder_defaults_match_v5r1_vector() {
        let tx = TransactionBuilder::new::<TON>()
            .public_key(vector_pubkey())
            .seqno(1)
            .valid_until(1_735_689_600)
            .add_message(vector_transfer())
            .build();
        assert_eq!(tx.wallet_version, WalletVersion::V5R1);
        assert_eq!(tx.workchain, 0);
        assert_eq!(tx.wallet_id, 0x7FFF_FF11);
        assert!(!tx.deploy);
        assert_eq!(
            hex::encode(tx.build_for_signing()),
            "0b45a74ddb8c4bd8f9a504b255869daad1bba81a1bd03351ea5ca1a3c47a2416"
        );
    }

    /// Explicit v4r2 configuration matches the spec's v4r2 payload.
    #[test]
    fn test_builder_v4r2_matches_vector() {
        let mut message = vector_transfer();
        message.mode = 1;
        let tx = TonTransactionBuilder::new()
            .wallet_version(WalletVersion::V4R2)
            .public_key(vector_pubkey())
            .seqno(1)
            .valid_until(1_735_689_600)
            .messages(vec![message])
            .build();
        assert_eq!(tx.wallet_id, 698_983_191);
        assert_eq!(
            hex::encode(tx.build_for_signing()),
            "84d81f593253be4c06ab026b2e44e78fc5d9a98a634021fd68b9519d1e8f4b83"
        );
    }

    #[test]
    #[should_panic(expected = "public_key is mandatory")]
    fn test_builder_requires_public_key() {
        let _ = TonTransactionBuilder::new()
            .seqno(0)
            .valid_until(1_735_689_600)
            .build();
    }

    #[test]
    #[should_panic(expected = "valid_until is mandatory")]
    fn test_builder_requires_valid_until() {
        let _ = TonTransactionBuilder::new()
            .public_key([0u8; 32])
            .seqno(0)
            .build();
    }

    #[test]
    #[should_panic(expected = "seqno is mandatory")]
    fn test_builder_requires_seqno() {
        let _ = TonTransactionBuilder::new()
            .public_key([0u8; 32])
            .valid_until(1_735_689_600)
            .build();
    }

    #[test]
    #[should_panic(expected = "fitting in u32")]
    fn test_builder_rejects_oversized_valid_until() {
        let _ = TonTransactionBuilder::new()
            .public_key([0u8; 32])
            .seqno(0)
            .valid_until(u64::MAX)
            .build();
    }

    #[test]
    fn test_builder_deploy_and_wallet_id_overrides() {
        let tx = TonTransactionBuilder::new()
            .public_key(vector_pubkey())
            .wallet_id(42)
            .workchain(-1)
            .seqno(0)
            .valid_until(1_735_689_600)
            .deploy(true)
            .build();
        assert_eq!(tx.wallet_id, 42);
        assert_eq!(tx.workchain, -1);
        assert!(tx.deploy);
        assert_eq!(tx.wallet_address().workchain, -1);
    }
}
