//! Utility functions for Solana transactions.
use super::constants::{SYSTEM_INSTRUCTION_TRANSFER, SYSTEM_PROGRAM_ID};
use super::types::{AccountMeta, Instruction, SolanaAddress};

/// Builds a System program `Transfer` instruction moving `lamports` from
/// `from` to `to`.
///
/// The instruction data is the bincode encoding of the System program's
/// `Transfer` variant: the `u32` little-endian discriminant `2` followed by
/// the lamport amount as a `u64` little-endian (12 bytes total).
///
/// ###### Example:
///
/// ```rust
/// use omni_transaction::solana::types::SolanaAddress;
/// use omni_transaction::solana::utils::system_transfer;
///
/// let from = SolanaAddress::from_base58("4vJ9JU1bJJE96FWSJKvHsmmFADCg4gpZQff4P3bkLKi").unwrap();
/// let to = SolanaAddress::from_base58("8opHzTAnfzRpPEx21XtnrVTX28YQuCpAjcn1PczScKh").unwrap();
///
/// let instruction = system_transfer(from, to, 1_000_000);
/// assert_eq!(hex::encode(&instruction.data), "0200000040420f0000000000");
/// ```
pub fn system_transfer(from: SolanaAddress, to: SolanaAddress, lamports: u64) -> Instruction {
    let mut data = Vec::with_capacity(12);
    data.extend_from_slice(&SYSTEM_INSTRUCTION_TRANSFER.to_le_bytes());
    data.extend_from_slice(&lamports.to_le_bytes());

    Instruction {
        program_id: SYSTEM_PROGRAM_ID,
        accounts: vec![AccountMeta::new(from, true), AccountMeta::new(to, false)],
        data,
    }
}

/// Decodes a base58 string into a fixed-size byte array.
pub(crate) fn decode_base58_fixed<const N: usize>(s: &str) -> Result<[u8; N], String> {
    let bytes = bs58::decode(s)
        .into_vec()
        .map_err(|e| format!("invalid base58 string: {e}"))?;
    let len = bytes.len();
    bytes
        .try_into()
        .map_err(|_| format!("invalid length: expected {N} bytes, got {len}"))
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn test_system_transfer_against_solana_system_interface() {
        let from =
            SolanaAddress::from_base58("4vJ9JU1bJJE96FWSJKvHsmmFADCg4gpZQff4P3bkLKi").unwrap();
        let to = SolanaAddress::from_base58("8opHzTAnfzRpPEx21XtnrVTX28YQuCpAjcn1PczScKh").unwrap();

        let ours = system_transfer(from, to, 1_000_000);

        let reference = solana_system_interface::instruction::transfer(
            &solana_pubkey::Pubkey::from_str("4vJ9JU1bJJE96FWSJKvHsmmFADCg4gpZQff4P3bkLKi")
                .unwrap(),
            &solana_pubkey::Pubkey::from_str("8opHzTAnfzRpPEx21XtnrVTX28YQuCpAjcn1PczScKh")
                .unwrap(),
            1_000_000,
        );

        assert_eq!(ours.data, reference.data);
        assert_eq!(hex::encode(&ours.data), "0200000040420f0000000000");
        assert_eq!(ours.program_id.to_bytes(), reference.program_id.to_bytes());
        assert_eq!(ours.accounts.len(), reference.accounts.len());
        for (our_meta, ref_meta) in ours.accounts.iter().zip(reference.accounts.iter()) {
            assert_eq!(our_meta.pubkey.to_bytes(), ref_meta.pubkey.to_bytes());
            assert_eq!(our_meta.is_signer, ref_meta.is_signer);
            assert_eq!(our_meta.is_writable, ref_meta.is_writable);
        }
    }
}
