#[cfg(feature = "borsh")]
use borsh::{BorshDeserialize, BorshSerialize};
#[cfg(feature = "schemars")]
use schemars::JsonSchema;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::bitcoin::types::ScriptBuf;

/// The previous output (coin) spent by a transparent input.
///
/// The [ZIP-244] signature digest commits to the value ([ZIP-244] S.2c) and
/// `scriptPubKey` ([ZIP-244] S.2d, S.2g) of **every** coin spent by the
/// transaction, so callers must supply one `SpentUtxo` per input, in input
/// order. Wrong or stale UTXO data produces a signature that fails validation
/// only at broadcast time.
///
/// [ZIP-244]: https://zips.z.cash/zip-0244
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "borsh", derive(BorshSerialize, BorshDeserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct SpentUtxo {
    /// The value of the spent coin in zatoshis.
    ///
    /// Committed to the signature digest as a **signed** 64-bit little-endian
    /// integer (consensus treats amounts as `i64 >= 0`).
    pub value: i64,
    /// The `scriptPubKey` of the spent coin.
    ///
    /// Note: for P2SH coins this is still the `scriptPubKey`, not the redeem
    /// script ([ZIP-244] S.2g note).
    pub script_pubkey: ScriptBuf,
}

impl SpentUtxo {
    /// Creates a new `SpentUtxo` from a value in zatoshis and the spent coin's `scriptPubKey`.
    pub const fn new(value: i64, script_pubkey: ScriptBuf) -> Self {
        Self {
            value,
            script_pubkey,
        }
    }
}
