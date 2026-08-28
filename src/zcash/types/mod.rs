//! Minimal required Zcash types, plus re-exports of the Bitcoin types shared
//! by the transparent part of the v5 transaction format (bit-identical wire
//! encoding).
mod consensus_branch_id;
mod sighash;
mod spent_utxo;

pub use self::consensus_branch_id::ConsensusBranchId;
pub use self::sighash::ZcashSighashType;
pub use self::spent_utxo::SpentUtxo;

// The transparent inputs/outputs of a Zcash v5 transaction are serialized
// exactly like Bitcoin's, so the Bitcoin types are reused directly.
pub use crate::bitcoin::types::Amount;
pub use crate::bitcoin::types::Hash;
pub use crate::bitcoin::types::OutPoint;
pub use crate::bitcoin::types::ScriptBuf;
pub use crate::bitcoin::types::Sequence;
pub use crate::bitcoin::types::TxIn;
pub use crate::bitcoin::types::TxOut;
pub use crate::bitcoin::types::Txid;
pub use crate::bitcoin::types::Witness;
