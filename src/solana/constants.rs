//! Solana protocol constants.
use super::types::SolanaAddress;

/// The System program id (`11111111111111111111111111111111`): all zeros.
pub const SYSTEM_PROGRAM_ID: SolanaAddress = SolanaAddress([0u8; 32]);

/// The prefix byte of a versioned message (`0x80 | version`); only version 0
/// exists, so a versioned message always starts with `0x80`. A first byte
/// without the high bit set means a legacy message.
pub const MESSAGE_VERSION_PREFIX: u8 = 0x80;

/// Maximum size in bytes of a serialized signed transaction that Solana
/// validators accept over the wire (1280-byte IPv6 MTU minus 48 bytes of
/// headers). Larger transactions are unbroadcastable.
pub const PACKET_DATA_SIZE: usize = 1232;

/// The `u32` little-endian discriminant of the System program `Transfer`
/// instruction.
pub const SYSTEM_INSTRUCTION_TRANSFER: u32 = 2;
