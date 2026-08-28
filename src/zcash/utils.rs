//! Utility functions for building Zcash transparent script sigs.
use super::types::ScriptBuf;

/// Maximum length of data pushable with a single direct push opcode.
const MAX_DIRECT_PUSH: usize = 75;
/// `OP_PUSHDATA1`: the next byte is the number of bytes to push.
const OP_PUSHDATA1: u8 = 0x4c;

/// Appends a minimal push of `data` to `buffer` (direct push up to 75 bytes,
/// `OP_PUSHDATA1` up to 255 bytes).
///
/// # Panics
///
/// If `data` is longer than 255 bytes.
fn push_slice(buffer: &mut Vec<u8>, data: &[u8]) {
    if data.len() > MAX_DIRECT_PUSH {
        assert!(
            data.len() <= u8::MAX as usize,
            "pushes longer than 255 bytes are not supported"
        );
        buffer.push(OP_PUSHDATA1);
    }
    buffer.push(data.len() as u8);
    buffer.extend_from_slice(data);
}

/// Builds a P2PKH script sig: `PUSH(sig_with_hashtype) PUSH(pubkey)`.
///
/// * `sig_with_hashtype`: the strict-DER, low-S encoded ECDSA signature with
///   the 1-byte `hash_type` (e.g. `0x01` for `SIGHASH_ALL`) appended,
///   typically 71-73 bytes.
/// * `pubkey`: the SEC1-encoded public key (33-byte compressed or 65-byte
///   uncompressed) matching the spent output's pubkey hash.
///
/// # Panics
///
/// If either argument is longer than 255 bytes.
pub fn p2pkh_script_sig(sig_with_hashtype: &[u8], pubkey: &[u8]) -> ScriptBuf {
    let mut script = Vec::with_capacity(2 + sig_with_hashtype.len() + pubkey.len());
    push_slice(&mut script, sig_with_hashtype);
    push_slice(&mut script, pubkey);
    ScriptBuf(script)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 33-byte compressed SEC1 public key.
    const PUBKEY_HEX: &str = "023333333333333333333333333333333333333333333333333333333333333333";

    /// 71-byte strict-DER signature (`30 45 02 21 <33 x 0x11> 02 20 <32 x
    /// 0x22>`) + 1-byte hash type (`0x01`).
    fn sig_with_hashtype() -> Vec<u8> {
        let mut sig = vec![0x30, 0x45, 0x02, 0x21];
        sig.extend_from_slice(&[0x11; 33]);
        sig.extend_from_slice(&[0x02, 0x20]);
        sig.extend_from_slice(&[0x22; 32]);
        sig.push(0x01);
        sig
    }

    #[test]
    fn test_p2pkh_script_sig() {
        let sig_with_hashtype = sig_with_hashtype();
        let pubkey = hex::decode(PUBKEY_HEX).unwrap();
        assert_eq!(sig_with_hashtype.len(), 72);
        assert_eq!(pubkey.len(), 33);

        let script_sig = p2pkh_script_sig(&sig_with_hashtype, &pubkey);

        let expected = format!("48{}21{PUBKEY_HEX}", hex::encode(&sig_with_hashtype));
        assert_eq!(hex::encode(&script_sig.0), expected);
    }

    #[test]
    fn test_push_slice_uses_pushdata1_above_75_bytes() {
        let mut buffer = Vec::new();
        push_slice(&mut buffer, &[0xAA; 76]);
        assert_eq!(buffer[0], OP_PUSHDATA1);
        assert_eq!(buffer[1], 76);
        assert_eq!(buffer.len(), 78);
    }

    #[test]
    #[should_panic(expected = "pushes longer than 255 bytes are not supported")]
    fn test_push_slice_panics_above_255_bytes() {
        let mut buffer = Vec::new();
        push_slice(&mut buffer, &[0xAA; 256]);
    }
}
