#[cfg(feature = "schemars")]
use schemars::JsonSchema;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use super::address::SolanaAddress;

/// Describes how an instruction uses one account.
///
/// The flags of the same address across all instructions are OR-ed together
/// when the message is compiled, and the fee payer is always forced to
/// signer + writable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct AccountMeta {
    /// The address of the account.
    pub pubkey: SolanaAddress,
    /// Whether the account must sign the transaction.
    pub is_signer: bool,
    /// Whether the instruction may mutate the account.
    pub is_writable: bool,
}

impl AccountMeta {
    /// Meta for a writable account.
    pub const fn new(pubkey: SolanaAddress, is_signer: bool) -> Self {
        Self {
            pubkey,
            is_signer,
            is_writable: true,
        }
    }

    /// Meta for a read-only account.
    pub const fn new_readonly(pubkey: SolanaAddress, is_signer: bool) -> Self {
        Self {
            pubkey,
            is_signer,
            is_writable: false,
        }
    }
}

/// A single program invocation: the program to call, the accounts it may
/// read/write, and its input data.
///
/// This is the *uncompiled* form used as builder input; compiling a message
/// turns it into a [`CompiledInstruction`](super::CompiledInstruction) whose
/// account references are indexes into the message's account keys.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct Instruction {
    /// The address of the program to invoke.
    pub program_id: SolanaAddress,
    /// The accounts the instruction reads or writes.
    pub accounts: Vec<AccountMeta>,
    /// The program input data.
    pub data: Vec<u8>,
}

/// The relevant content of an on-chain address lookup table, used when
/// compiling a V0 message.
///
/// During compilation, an account address is replaced by a 1-byte index into
/// `addresses` when it is not a signer, not an invoked program id, and appears
/// in the table.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct AddressLookupTableAccount {
    /// The address of the lookup table account itself.
    pub key: SolanaAddress,
    /// The addresses stored in the table, in on-chain order.
    pub addresses: Vec<SolanaAddress>,
}
