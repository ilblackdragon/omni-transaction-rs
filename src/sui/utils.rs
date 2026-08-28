//! Utility functions for hashing and Sui address derivation.
use blake2::digest::consts::U32;
use blake2::{Blake2b, Digest};

use super::types::{SignatureScheme, SuiAddress};

/// Blake2b parameterized to a 32-byte digest — Sui's `blake2b-256`.
///
/// This is **not** Blake2b-512 truncated to 32 bytes; the two produce
/// completely different digests.
type Blake2b256 = Blake2b<U32>;

/// Computes the Blake2b-256 hash of `data`.
pub fn blake2b256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Blake2b256::new();
    hasher.update(data);
    hasher.finalize().into()
}

/// Parses a Sui address from a hex string, with or without a `0x` prefix.
/// Short forms such as `"0x2"` are zero-padded on the left.
///
/// # Panics
///
/// Panics if the input is empty, longer than 64 hex characters or contains
/// non-hex characters. Use [`SuiAddress::from_hex`] for a fallible parse.
pub fn parse_sui_address(address: &str) -> SuiAddress {
    SuiAddress::from_hex(address).expect("address should be at most 64 hex characters")
}

/// Derives the Sui address controlled by `public_key`:
/// `blake2b256(scheme_flag || public_key)`.
///
/// The transaction sender must equal the address derived from the public key
/// that signs it (32-byte key for ed25519, 33-byte compressed key for
/// secp256k1/secp256r1).
///
/// # Panics
///
/// Panics if `public_key` does not have the exact length the scheme
/// requires.
pub fn derive_sui_address(scheme: SignatureScheme, public_key: &[u8]) -> SuiAddress {
    assert_eq!(
        public_key.len(),
        scheme.public_key_length(),
        "public key length does not match the signature scheme"
    );
    let mut hasher = Blake2b256::new();
    hasher.update([scheme.flag()]);
    hasher.update(public_key);
    SuiAddress::new(hasher.finalize().into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blake2b256_is_parameterized_not_truncated() {
        // RFC 7693 test: blake2b-512("abc") starts with ba80a53f...; the
        // 32-byte-parameterized hash of "abc" is a different digest.
        let digest = blake2b256(b"abc");
        assert_eq!(
            hex::encode(digest),
            "bddd813c634239723171ef3fee98579b94964e3bb1cb3e427262c8c068d52319"
        );
    }

    #[test]
    fn test_parse_sui_address() {
        let address = parse_sui_address("0x2");
        assert_eq!(
            address.to_string(),
            "0x0000000000000000000000000000000000000000000000000000000000000002"
        );
    }

    #[test]
    #[should_panic(expected = "address should be at most 64 hex characters")]
    fn test_parse_sui_address_panics_on_invalid_input() {
        parse_sui_address("not-hex");
    }

    #[test]
    fn test_derive_ed25519_address_against_reference_sdk() {
        let public_key = [0x2au8; 32];
        let ours = derive_sui_address(SignatureScheme::Ed25519, &public_key);
        let reference = sui_sdk_types::Ed25519PublicKey::new(public_key).derive_address();
        assert_eq!(ours.into_inner(), reference.into_inner());
    }

    #[test]
    fn test_derive_secp256k1_address_against_reference_sdk() {
        let mut public_key = [0x11u8; 33];
        public_key[0] = 0x02; // valid compressed-point prefix
        let ours = derive_sui_address(SignatureScheme::Secp256k1, &public_key);
        let reference = sui_sdk_types::Secp256k1PublicKey::new(public_key).derive_address();
        assert_eq!(ours.into_inner(), reference.into_inner());
    }

    #[test]
    #[should_panic(expected = "public key length does not match")]
    fn test_derive_sui_address_panics_on_wrong_length() {
        derive_sui_address(SignatureScheme::Ed25519, &[0u8; 33]);
    }
}
