#[cfg(feature = "schemars")]
use schemars::JsonSchema;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use super::address::{Blockhash, SolanaAddress};
use crate::solana::constants::MESSAGE_VERSION_PREFIX;
use crate::solana::encoding::encode_length;

/// The fixed 3-byte header of a Solana message.
///
/// The first `num_required_signatures` entries of the account keys are the
/// signers; `signature[i]` of the transaction is produced by
/// `account_keys[i]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct MessageHeader {
    /// Number of accounts that must sign the transaction.
    pub num_required_signatures: u8,
    /// Number of read-only accounts at the end of the signer segment.
    pub num_readonly_signed_accounts: u8,
    /// Number of read-only accounts at the end of the whole key array.
    pub num_readonly_unsigned_accounts: u8,
}

/// An instruction whose program and accounts are 1-byte indexes into the
/// message's account keys (for V0 messages: into the combined static +
/// lookup address space).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct CompiledInstruction {
    /// Index of the program id in the account keys.
    pub program_id_index: u8,
    /// Indexes of the instruction's accounts in the account keys.
    pub accounts: Vec<u8>,
    /// The program input data.
    pub data: Vec<u8>,
}

/// A compiled reference to an on-chain address lookup table (V0 messages
/// only): the table's address plus the indexes of the entries loaded as
/// writable and as read-only.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct MessageAddressTableLookup {
    /// The address of the lookup table account.
    pub account_key: SolanaAddress,
    /// Indexes (into the table) of the addresses loaded as writable.
    pub writable_indexes: Vec<u8>,
    /// Indexes (into the table) of the addresses loaded as read-only.
    pub readonly_indexes: Vec<u8>,
}

/// A Solana message: the exact bytes that get signed.
///
/// `Legacy` messages serialize without any prefix; `V0` messages are prefixed
/// with the version byte `0x80` and append an address-table-lookup section.
/// At runtime the account index space of a V0 message is the concatenation of
/// the static keys, then every writable lookup address (in table order), then
/// every read-only lookup address.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub enum SolanaMessage {
    /// The original message format (no version prefix).
    Legacy {
        /// The 3-byte message header.
        header: MessageHeader,
        /// All account keys: signers first, then non-signers, each bucket
        /// with writable accounts before read-only ones.
        account_keys: Vec<SolanaAddress>,
        /// A recent blockhash (expires after ~150 slots).
        recent_blockhash: Blockhash,
        /// The compiled instructions.
        instructions: Vec<CompiledInstruction>,
    },
    /// The versioned message format (version 0), supporting address lookup
    /// tables.
    V0 {
        /// The 3-byte message header (over the static keys only).
        header: MessageHeader,
        /// The static account keys (signers and invoked programs always stay
        /// here; other keys may be demoted into lookup tables).
        account_keys: Vec<SolanaAddress>,
        /// A recent blockhash (expires after ~150 slots).
        recent_blockhash: Blockhash,
        /// The compiled instructions (indexes cover static + lookup keys).
        instructions: Vec<CompiledInstruction>,
        /// The address table lookups.
        address_table_lookups: Vec<MessageAddressTableLookup>,
    },
}

impl SolanaMessage {
    /// Serializes the message to its wire format — exactly the bytes that
    /// must be signed with ed25519 (no additional hashing).
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(256);
        match self {
            Self::Legacy {
                header,
                account_keys,
                recent_blockhash,
                instructions,
            } => {
                encode_common_sections(
                    &mut out,
                    header,
                    account_keys,
                    recent_blockhash,
                    instructions,
                );
            }
            Self::V0 {
                header,
                account_keys,
                recent_blockhash,
                instructions,
                address_table_lookups,
            } => {
                out.push(MESSAGE_VERSION_PREFIX);
                encode_common_sections(
                    &mut out,
                    header,
                    account_keys,
                    recent_blockhash,
                    instructions,
                );
                encode_length(address_table_lookups.len(), &mut out);
                for lookup in address_table_lookups {
                    out.extend_from_slice(&lookup.account_key.0);
                    encode_length(lookup.writable_indexes.len(), &mut out);
                    out.extend_from_slice(&lookup.writable_indexes);
                    encode_length(lookup.readonly_indexes.len(), &mut out);
                    out.extend_from_slice(&lookup.readonly_indexes);
                }
            }
        }
        out
    }

    /// The message header.
    pub const fn header(&self) -> &MessageHeader {
        match self {
            Self::Legacy { header, .. } | Self::V0 { header, .. } => header,
        }
    }

    /// The static account keys of the message.
    pub const fn account_keys(&self) -> &Vec<SolanaAddress> {
        match self {
            Self::Legacy { account_keys, .. } | Self::V0 { account_keys, .. } => account_keys,
        }
    }

    /// The number of signatures required for the message.
    pub const fn num_required_signatures(&self) -> u8 {
        self.header().num_required_signatures
    }
}

/// Encodes header + account keys + blockhash + instructions (identical for
/// legacy and V0 messages).
fn encode_common_sections(
    out: &mut Vec<u8>,
    header: &MessageHeader,
    account_keys: &[SolanaAddress],
    recent_blockhash: &Blockhash,
    instructions: &[CompiledInstruction],
) {
    out.push(header.num_required_signatures);
    out.push(header.num_readonly_signed_accounts);
    out.push(header.num_readonly_unsigned_accounts);

    encode_length(account_keys.len(), out);
    for key in account_keys {
        out.extend_from_slice(&key.0);
    }

    out.extend_from_slice(&recent_blockhash.0);

    encode_length(instructions.len(), out);
    for instruction in instructions {
        out.push(instruction.program_id_index);
        encode_length(instruction.accounts.len(), out);
        out.extend_from_slice(&instruction.accounts);
        encode_length(instruction.data.len(), out);
        out.extend_from_slice(&instruction.data);
    }
}
