//! Wallet contract versions and internal transfer messages.
use core::fmt;
use std::sync::OnceLock;

#[cfg(feature = "schemars")]
use schemars::JsonSchema;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use super::address::TonAddress;
use super::boc::parse_boc_single_root;
use super::cell::{Cell, CellBuilder, CellError, MAX_CELL_REFS};
use super::coins::Coins;
use crate::ton::constants::{
    WALLET_V4R2_CODE_BOC, WALLET_V4R2_CODE_HASH, WALLET_V5R1_CODE_BOC, WALLET_V5R1_CODE_HASH,
};

/// TON network global identifier of mainnet (used in v5r1 wallet ids).
pub const MAINNET_GLOBAL_ID: i32 = -239;
/// TON network global identifier of testnet (used in v5r1 wallet ids).
pub const TESTNET_GLOBAL_ID: i32 = -3;
/// Default `subwallet_id` of v4r2 wallets (`698983191 = 0x29A9A317`).
pub const DEFAULT_V4R2_WALLET_ID: u32 = 698_983_191;

/// Default send mode of an internal transfer:
/// `PAY_GAS_SEPARATELY (1) | IGNORE_ERRORS (2)`.
pub const DEFAULT_SEND_MODE: u8 = 3;

/// Computes a v5r1 (W5) 32-bit wallet id: the network global id XOR-ed with
/// the packed client context (`1` bit, workchain `int8`, wallet version `u8`
/// = 0, subwallet `u15`).
///
/// The mainnet workchain-0 default is `2147483409 = 0x7FFFFF11`.
///
/// # Panics
///
/// Panics if `subwallet` does not fit in 15 bits.
pub const fn v5r1_wallet_id(network_global_id: i32, workchain: i8, subwallet: u16) -> u32 {
    assert!(subwallet < 1 << 15, "v5r1 subwallet must fit in 15 bits");
    let context: u32 =
        1 << 31 | (workchain as u8 as u32) << 23 | /* version 0 << 15 | */ subwallet as u32;
    network_global_id as u32 ^ context
}

/// Supported wallet contract versions.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub enum WalletVersion {
    /// Wallet v4r2 (`ton-blockchain/wallet-contract`), the largest deployed
    /// base.
    V4R2,
    /// Wallet v5r1 / W5 (`ton-blockchain/wallet-contract-v5`), the current
    /// standard.
    #[default]
    V5R1,
}

impl WalletVersion {
    /// Returns the pinned representation hash of the official code cell for
    /// this version.
    pub const fn code_hash(self) -> [u8; 32] {
        match self {
            Self::V4R2 => WALLET_V4R2_CODE_HASH,
            Self::V5R1 => WALLET_V5R1_CODE_HASH,
        }
    }

    /// Returns the official wallet code cell for this version (embedded in
    /// the library; its hash equals [`Self::code_hash`]).
    ///
    /// The ~1.4 KB code BoC is parsed and hash-verified once per version and
    /// then cached, so repeated calls (address derivation, `StateInit`
    /// building) cost a shallow clone instead of a full re-parse plus
    /// SHA-256 over every cell — the child cells stay shared behind `Arc`.
    ///
    /// # Panics
    ///
    /// Panics on the first call for a version if the embedded code BoC is
    /// not valid hex, is not a valid single-root Bag of Cells, or does not
    /// hash to [`Self::code_hash`] (all impossible for an unmodified build).
    pub fn code_cell(self) -> Cell {
        static V4R2_CODE_CELL: OnceLock<Cell> = OnceLock::new();
        static V5R1_CODE_CELL: OnceLock<Cell> = OnceLock::new();

        let (cache, boc) = match self {
            Self::V4R2 => (&V4R2_CODE_CELL, WALLET_V4R2_CODE_BOC),
            Self::V5R1 => (&V5R1_CODE_CELL, WALLET_V5R1_CODE_BOC),
        };
        cache
            .get_or_init(|| {
                let bytes = hex::decode(boc).expect("embedded wallet code is valid hex");
                let cell =
                    parse_boc_single_root(&bytes).expect("embedded wallet code is a valid BoC");
                assert!(
                    cell.repr_hash() == self.code_hash(),
                    "embedded wallet code hash mismatch"
                );
                cell
            })
            .clone()
    }

    /// Returns the conventional default wallet id for this version on the
    /// given workchain: `698983191` for v4r2, the mainnet
    /// [`v5r1_wallet_id`] with subwallet 0 for v5r1 (`0x7FFFFF11` on
    /// workchain 0).
    pub const fn default_wallet_id(self, workchain: i8) -> u32 {
        match self {
            Self::V4R2 => DEFAULT_V4R2_WALLET_ID,
            Self::V5R1 => v5r1_wallet_id(MAINNET_GLOBAL_ID, workchain, 0),
        }
    }

    /// Builds the initial (pre-deployment) data cell for this version.
    ///
    /// - v4r2: `seqno u32 = 0 | subwallet_id u32 | public_key 256 bits |
    ///   empty plugins dict '0'`.
    /// - v5r1: `is_signature_allowed '1' | seqno u32 = 0 | wallet_id u32 |
    ///   public_key 256 bits | empty extensions dict '0'`.
    pub fn initial_data_cell(self, wallet_id: u32, public_key: &[u8; 32]) -> Cell {
        let builder = match self {
            Self::V4R2 => CellBuilder::new().store_u32(0),
            Self::V5R1 => CellBuilder::new()
                .store_bit(true)
                .and_then(|b| b.store_u32(0)),
        };
        builder
            .and_then(|b| b.store_u32(wallet_id))
            .and_then(|b| b.store_slice(public_key))
            .and_then(|b| b.store_bit(false))
            .and_then(CellBuilder::build)
            .expect("initial wallet data always fits in one cell")
    }

    /// Builds the wallet `StateInit` cell: bits `00110` plus references to
    /// the code and initial data cells.
    pub fn state_init_cell(self, wallet_id: u32, public_key: &[u8; 32]) -> Cell {
        CellBuilder::new()
            .store_uint(0b00110, 5)
            .and_then(|b| b.store_ref(self.code_cell()))
            .and_then(|b| b.store_ref(self.initial_data_cell(wallet_id, public_key)))
            .and_then(CellBuilder::build)
            .expect("state init always fits in one cell")
    }
}

/// One internal transfer carried by a wallet transaction: `dest` receives
/// `value` nanotons plus an optional message body.
///
/// ###### Example:
///
/// ```rust
/// use omni_transaction::ton::types::{Coins, InternalMessage, TonAddress};
///
/// let dest: TonAddress = "EQCD39VS5jcptHL8vMjEXrzGaRcCVYto7HUn4bpAOg8xqB2N"
///     .parse()
///     .unwrap();
/// let message = InternalMessage::new(dest, Coins::from_nano(50_000_000))
///     .with_comment("thanks!")
///     .unwrap();
/// assert!(message.bounce);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct InternalMessage {
    /// Recipient address.
    pub dest: TonAddress,
    /// Amount to transfer, in nanotons.
    pub value: Coins,
    /// Whether the transfer bounces back on failure (recommended `true`;
    /// use `false` only when topping up not-yet-deployed addresses).
    #[cfg_attr(feature = "serde", serde(default = "default_bounce"))]
    pub bounce: bool,
    /// Raw send mode (defaults to [`DEFAULT_SEND_MODE`]). For v5r1 wallets
    /// the transaction builder force-ORs `IGNORE_ERRORS (+2)` in, as the
    /// contract requires for externally-authorized requests.
    #[cfg_attr(feature = "serde", serde(default = "default_mode"))]
    pub mode: u8,
    /// Optional message body cell (e.g. a text comment or a jetton payload).
    #[cfg_attr(feature = "serde", serde(default))]
    pub body: Option<Cell>,
}

#[cfg(feature = "serde")]
const fn default_bounce() -> bool {
    true
}

#[cfg(feature = "serde")]
const fn default_mode() -> u8 {
    DEFAULT_SEND_MODE
}

impl InternalMessage {
    /// Creates a bounceable transfer with the default send mode and no body.
    pub const fn new(dest: TonAddress, value: Coins) -> Self {
        Self {
            dest,
            value,
            bounce: true,
            mode: DEFAULT_SEND_MODE,
            body: None,
        }
    }

    /// Attaches a UTF-8 text comment as the message body (`u32` opcode 0
    /// followed by the text).
    ///
    /// # Errors
    ///
    /// Returns [`CellError::CellOverflow`] if the comment does not fit in a
    /// single cell (at most 123 bytes).
    pub fn with_comment(mut self, comment: &str) -> Result<Self, CellError> {
        self.body = Some(crate::ton::utils::comment_cell(comment)?);
        Ok(self)
    }

    /// Encodes this transfer as a `MessageRelaxed` cell (`int_msg_info$0`,
    /// `ihr_disabled` set, `addr_none` source, zero fees, zero
    /// `created_lt`/`created_at`, no init; the body is inlined when it fits,
    /// otherwise stored as a reference — matching reference tooling).
    ///
    /// # Errors
    ///
    /// Returns a [`CellError`] if the transfer cannot be encoded (e.g. the
    /// value exceeds the `VarUInteger 16` range).
    pub fn to_cell(&self) -> Result<Cell, CellError> {
        let mut builder = CellBuilder::new()
            .store_bit(false)? // int_msg_info$0
            .store_bit(true)? // ihr_disabled
            .store_bit(self.bounce)? // bounce
            .store_bit(false)? // bounced
            .store_uint(0, 2)? // src: addr_none$00
            .store_address(&self.dest)? // dest: addr_std$10
            .store_coins(self.value)? // value: Grams
            .store_bit(false)? // no extra currencies
            .store_uint(0, 4)? // ihr_fee: Grams 0
            .store_uint(0, 4)? // fwd_fee: Grams 0
            .store_u64(0)? // created_lt
            .store_u32(0)? // created_at
            .store_bit(false)?; // init: nothing
        builder = match &self.body {
            None => builder.store_bit(false)?, // body: inlined and empty
            Some(body) => {
                // One spare bit for the Either tag, per the reference
                // inlining rule.
                if usize::from(builder.remaining_bits()) > usize::from(body.bit_len())
                    && builder.refs_count() + body.refs().len() <= MAX_CELL_REFS
                {
                    builder.store_bit(false)?.store_cell(body)?
                } else {
                    builder.store_bit(true)?.store_ref(body.clone())?
                }
            }
        };
        builder.build()
    }
}

impl fmt::Display for InternalMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} nanoton -> {}", self.value, self.dest)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::ton::types::boc::serialize_boc;

    fn vector_pubkey() -> [u8; 32] {
        hex::decode("31debe55d37c722768b137131caa6087080b2e0b60b94bd785d14575cfa498bc")
            .unwrap()
            .try_into()
            .unwrap()
    }

    fn vector_dest() -> TonAddress {
        "0:83dfd552e63729b472fcbcc8c45ebcc6691702558b68ec7527e1ba403a0f31a8"
            .parse()
            .unwrap()
    }

    /// Spec vector: default mainnet workchain-0 v5r1 wallet id, plus the
    /// testnet value pinned by reference implementations.
    #[test]
    fn test_v5r1_wallet_id_vectors() {
        assert_eq!(v5r1_wallet_id(MAINNET_GLOBAL_ID, 0, 0), 0x7FFF_FF11);
        assert_eq!(v5r1_wallet_id(MAINNET_GLOBAL_ID, 0, 0), 2_147_483_409);
        assert_eq!(v5r1_wallet_id(TESTNET_GLOBAL_ID, 0, 0), 0x7FFF_FFFD);
        assert_eq!(WalletVersion::V5R1.default_wallet_id(0), 0x7FFF_FF11);
        assert_eq!(WalletVersion::V4R2.default_wallet_id(0), 698_983_191);
    }

    /// The embedded code cells hash to the pinned official code hashes.
    #[test]
    fn test_embedded_code_cells_match_pinned_hashes() {
        for version in [WalletVersion::V4R2, WalletVersion::V5R1] {
            assert_eq!(version.code_cell().repr_hash(), version.code_hash());
        }
        assert_eq!(
            hex::encode(WalletVersion::V4R2.code_hash()),
            "feb5ff6820e2ff0d9483e7e0d62c817d846789fb4ae580c878866d959dabd5c0"
        );
        assert_eq!(
            hex::encode(WalletVersion::V5R1.code_hash()),
            "20834b7b72b112147e1b2fb457b84e74d1a30f04f737d4f62a668e9552d2b72f"
        );
    }

    /// The embedded code cell is parsed once per version: repeated calls
    /// return equal cells that share the very same child cells (`Arc`), which
    /// is only possible when the parse result is cached.
    #[test]
    fn test_code_cell_is_cached_per_version() {
        for version in [WalletVersion::V4R2, WalletVersion::V5R1] {
            let first = version.code_cell();
            let second = version.code_cell();
            assert_eq!(first, second);
            assert_eq!(first.repr_hash(), version.code_hash());
            assert_eq!(second.repr_hash(), version.code_hash());
            assert_eq!(first.refs().len(), second.refs().len());
            assert!(!first.refs().is_empty(), "wallet code has child cells");
            for (a, b) in first.refs().iter().zip(second.refs()) {
                assert!(
                    Arc::ptr_eq(a, b),
                    "child cells must be shared, i.e. the code cell must be cached"
                );
            }
        }
        // The two versions must not share a cache slot.
        assert_ne!(
            WalletVersion::V4R2.code_cell(),
            WalletVersion::V5R1.code_cell()
        );
    }

    /// Spec vectors: initial data cells for both versions.
    #[test]
    fn test_initial_data_cells_match_vectors() {
        let v4 = WalletVersion::V4R2.initial_data_cell(DEFAULT_V4R2_WALLET_ID, &vector_pubkey());
        assert_eq!(v4.bit_len(), 321);
        assert_eq!(
            hex::encode(serialize_boc(&v4, false)),
            "b5ee9c7201010101002b0000510000000029a9a31731debe55d37c722768b137131caa6087080b2e0b60b94bd785d14575cfa498bc40"
        );
        let v5 = WalletVersion::V5R1.initial_data_cell(0x7FFF_FF11, &vector_pubkey());
        assert_eq!(v5.bit_len(), 322);
        assert_eq!(
            hex::encode(serialize_boc(&v5, false)),
            "b5ee9c7201010101002b000051800000003fffff8898ef5f2ae9be3913b4589b898e55304384059705b05ca5ebc2e8a2bae7d24c5e20"
        );
    }

    /// Spec vectors: wallet addresses derived from the StateInit hash.
    #[test]
    fn test_state_init_addresses_match_vectors() {
        let v4 = WalletVersion::V4R2.state_init_cell(DEFAULT_V4R2_WALLET_ID, &vector_pubkey());
        assert_eq!(
            hex::encode(v4.repr_hash()),
            "ba272475e07d7228e1cdd4328ca3a5db0734c15c2ce7bd9184d2b701d1f5fa64"
        );
        let v5 = WalletVersion::V5R1.state_init_cell(0x7FFF_FF11, &vector_pubkey());
        assert_eq!(
            hex::encode(v5.repr_hash()),
            "58cd759e9e6d3a82eb0d464eb02d0c0cfc73bb42ce7eb550f090f5e9fa2c592f"
        );
    }

    /// Spec vector: the 416-bit single-transfer `MessageRelaxed` layout.
    #[test]
    fn test_internal_message_matches_vector() {
        let message = InternalMessage::new(vector_dest(), Coins::from_nano(50_000_000));
        let cell = message.to_cell().unwrap();
        assert_eq!(cell.bit_len(), 416);
        assert_eq!(
            hex::encode(cell.repr_hash()),
            "b04e90f0fcb255a8b4cca4854de622d098631af70f129823c56aec2cd9bd7567"
        );
        assert_eq!(
            hex::encode(serialize_boc(&cell, false)),
            "b5ee9c72010101010036000068620041efeaa9731b94da397e5e64622f5e63348b812ac5b4763a93f0dd201d0798d42017d7840000000000000000000000000000"
        );
    }

    /// Short comments are inlined into the message cell; long ones become a
    /// reference; over-long ones fail.
    #[test]
    fn test_comment_bodies() {
        let short = InternalMessage::new(vector_dest(), Coins::from_nano(50_000_000))
            .with_comment("hello TON")
            .unwrap();
        let cell = short.to_cell().unwrap();
        // 416 bits + 32-bit opcode + 9 bytes of text, everything inlined.
        assert_eq!(cell.bit_len(), 416 + 32 + 9 * 8);
        assert_eq!(cell.refs().len(), 0);

        let long_text = "x".repeat(100);
        let long = InternalMessage::new(vector_dest(), Coins::from_nano(50_000_000))
            .with_comment(&long_text)
            .unwrap();
        let cell = long.to_cell().unwrap();
        assert_eq!(cell.bit_len(), 416); // body spills into a reference
        assert_eq!(cell.refs().len(), 1);
        assert_eq!(cell.refs()[0].bit_len(), 32 + 100 * 8);

        let too_long = "x".repeat(124);
        assert_eq!(
            InternalMessage::new(vector_dest(), Coins::from_nano(1))
                .with_comment(&too_long)
                .unwrap_err(),
            CellError::CellOverflow
        );
    }
}
