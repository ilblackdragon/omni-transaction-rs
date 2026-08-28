//! Utility functions for TON: checksums, wallet address derivation and
//! comment bodies.
use super::types::{Cell, CellBuilder, CellError, TonAddress, WalletVersion};

/// Computes CRC-32C (Castagnoli): polynomial `0x1EDC6F41` (reflected
/// `0x82F63B78`), init and xorout `0xFFFFFFFF`, reflected input/output.
/// Used by the Bag of Cells checksum.
pub fn crc32c(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for &byte in data {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = crc >> 1 ^ 0x82F6_3B78 & mask;
        }
    }
    !crc
}

/// Computes CRC-16/XMODEM: polynomial `0x1021`, init 0, no reflection, no
/// xorout. Used by the user-friendly address checksum.
pub fn crc16_xmodem(data: &[u8]) -> u16 {
    let mut crc: u16 = 0;
    for &byte in data {
        crc ^= u16::from(byte) << 8;
        for _ in 0..8 {
            crc = if crc & 0x8000 == 0 {
                crc << 1
            } else {
                crc << 1 ^ 0x1021
            };
        }
    }
    crc
}

/// Builds a text-comment message body: `u32` opcode 0 followed by the UTF-8
/// bytes of `comment`.
///
/// # Errors
///
/// Returns [`CellError::CellOverflow`] if the comment does not fit in a
/// single cell (at most 123 bytes; longer comments would need snake-cell
/// continuation, which this library does not produce).
pub fn comment_cell(comment: &str) -> Result<Cell, CellError> {
    CellBuilder::new()
        .store_u32(0)?
        .store_slice(comment.as_bytes())?
        .build()
}

/// Derives the wallet address for a public key: the workchain plus the
/// representation hash of the wallet `StateInit`
/// (see [`WalletVersion::state_init_cell`]).
///
/// `wallet_id` must be the id the wallet is (or will be) deployed with;
/// pass [`WalletVersion::default_wallet_id`] for the conventional default.
pub fn derive_wallet_address(
    version: WalletVersion,
    workchain: i8,
    wallet_id: u32,
    public_key: &[u8; 32],
) -> TonAddress {
    TonAddress::new(
        workchain,
        version.state_init_cell(wallet_id, public_key).repr_hash(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crc32c_known_values() {
        // RFC 3720 test vector: 32 zero bytes.
        assert_eq!(crc32c(&[0u8; 32]), 0x8A91_36AA);
        // "123456789" -> 0xE3069283 (standard CRC-32C check value).
        assert_eq!(crc32c(b"123456789"), 0xE306_9283);
        assert_eq!(crc32c(b""), 0);
    }

    #[test]
    fn test_crc16_xmodem_known_values() {
        // "123456789" -> 0x31C3 (standard XMODEM check value).
        assert_eq!(crc16_xmodem(b"123456789"), 0x31C3);
        assert_eq!(crc16_xmodem(b""), 0);
    }

    #[test]
    fn test_comment_cell_layout() {
        let cell = comment_cell("hi").unwrap();
        assert_eq!(cell.bit_len(), 48);
        assert_eq!(cell.data(), &[0, 0, 0, 0, b'h', b'i']);
        assert!(comment_cell(&"y".repeat(123)).is_ok());
        assert_eq!(
            comment_cell(&"y".repeat(124)).unwrap_err(),
            CellError::CellOverflow
        );
    }

    /// Spec vectors: wallet addresses for the fixed test public key.
    #[test]
    fn test_derive_wallet_address_matches_vectors() {
        let public_key: [u8; 32] =
            hex::decode("31debe55d37c722768b137131caa6087080b2e0b60b94bd785d14575cfa498bc")
                .unwrap()
                .try_into()
                .unwrap();
        let v4 = derive_wallet_address(
            WalletVersion::V4R2,
            0,
            WalletVersion::V4R2.default_wallet_id(0),
            &public_key,
        );
        assert_eq!(
            v4.to_string(),
            "EQC6JyR14H1yKOHN1DKMo6XbBzTBXCznvZGE0rcB0fX6ZHPD"
        );
        let v5 = derive_wallet_address(
            WalletVersion::V5R1,
            0,
            WalletVersion::V5R1.default_wallet_id(0),
            &public_key,
        );
        assert_eq!(
            v5.to_string(),
            "EQBYzXWenm06gusNRk6wLQwM_HO7Qs5-tVDwkPXp-ixZL-0W"
        );
    }
}
