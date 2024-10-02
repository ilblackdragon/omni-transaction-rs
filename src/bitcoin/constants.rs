/// The marker MUST be a 1-byte zero value: 0x00. (BIP-141)
pub const SEGWIT_MARKER: u8 = 0x00;

/// The flag MUST be a 1-byte non-zero value. Currently, 0x01 MUST be used. (BIP-141)
pub const SEGWIT_FLAG: u8 = 0x01;
