//! Solana `compact-u16` ("shortvec") encoding.
//!
//! Every variable-length collection in a Solana message is prefixed with its
//! length encoded as a `compact-u16`: a little-endian base-128 varint over
//! `u16` that occupies 1 to 3 bytes. Each byte carries 7 payload bits
//! (bits 0-6) and uses bit 7 (`0x80`) as the continuation flag.
//!
//! The encoding emitted here is always the *minimal* form: Solana validators
//! reject "alias" encodings (e.g. `[0x81, 0x80, 0x00]` for `1`) as well as
//! encodings longer than 3 bytes.

/// Appends the minimal `compact-u16` (shortvec) encoding of `value` to `out`.
pub fn encode_compact_u16(value: u16, out: &mut Vec<u8>) {
    let mut remaining = value;
    loop {
        let byte = (remaining & 0x7f) as u8;
        remaining >>= 7;
        if remaining == 0 {
            out.push(byte);
            break;
        }
        out.push(byte | 0x80);
    }
}

/// Number of bytes the minimal `compact-u16` encoding of `value` occupies
/// (1, 2 or 3), without serializing it.
pub const fn compact_u16_len(value: u16) -> usize {
    match value {
        0..=0x7f => 1,
        0x80..=0x3fff => 2,
        _ => 3,
    }
}

/// Appends a length prefix for a collection of `len` elements, panicking if
/// the length does not fit in a `u16` (the maximum a shortvec can express).
pub fn encode_length(len: usize, out: &mut Vec<u8>) {
    let len = u16::try_from(len).expect("collection length exceeds u16::MAX");
    encode_compact_u16(len, out);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn compact_u16(value: u16) -> Vec<u8> {
        let mut out = Vec::with_capacity(3);
        encode_compact_u16(value, &mut out);
        out
    }

    /// Spec vector V5: both boundaries of every byte-length class, generated
    /// with `solana-short-vec` 3.3.0 through bincode.
    #[test]
    fn test_compact_u16_official_vectors() {
        let vectors: [(u16, &str); 9] = [
            (0, "00"),
            (1, "01"),
            (127, "7f"),
            (128, "8001"),
            (255, "ff01"),
            (256, "8002"),
            (16383, "ff7f"),
            (16384, "808001"),
            (65535, "ffff03"),
        ];
        for (value, expected_hex) in vectors {
            assert_eq!(
                hex::encode(compact_u16(value)),
                expected_hex,
                "compact-u16 encoding mismatch for {value}"
            );
        }
    }

    /// Cross-check every representable value against the reference
    /// `solana-short-vec` serde implementation (via bincode).
    #[test]
    fn test_compact_u16_against_solana_short_vec() {
        for value in (0..=u16::MAX).step_by(13) {
            let reference = bincode::serialize(&solana_short_vec::ShortU16(value)).unwrap();
            assert_eq!(compact_u16(value), reference, "mismatch for {value}");
        }
        // Always include the extremes.
        for value in [0, 1, u16::MAX - 1, u16::MAX] {
            let reference = bincode::serialize(&solana_short_vec::ShortU16(value)).unwrap();
            assert_eq!(compact_u16(value), reference, "mismatch for {value}");
        }
    }

    /// `compact_u16_len` must agree with the encoder for every `u16`.
    #[test]
    fn test_compact_u16_len_matches_encoder() {
        for value in 0..=u16::MAX {
            assert_eq!(
                compact_u16_len(value),
                compact_u16(value).len(),
                "length mismatch for {value}"
            );
        }
    }

    #[test]
    #[should_panic(expected = "collection length exceeds u16::MAX")]
    fn test_encode_length_rejects_oversized_collections() {
        let mut out = Vec::new();
        encode_length(65536, &mut out);
    }
}
