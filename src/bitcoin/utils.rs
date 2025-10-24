//! Utility functions for serialization and encoding of Bitcoin data structures

use alloc::vec;
use alloc::vec::Vec;

fn encode_signature_as_der(signature_bytes: &[u8]) -> Vec<u8> {
    assert_eq!(
        signature_bytes.len(),
        64,
        "Signature must be 64 bytes long (32 bytes for R and 32 bytes for S)"
    );

    let (r, s) = signature_bytes.split_at(32);
    let r_der = encode_asn1_integer(r);
    let s_der = encode_asn1_integer(s);

    let total_len = r_der.len() + s_der.len();

    let mut der = vec![0x30, total_len as u8];
    der.extend_from_slice(&r_der);
    der.extend_from_slice(&s_der);

    der
}

fn encode_asn1_integer(bytes: &[u8]) -> Vec<u8> {
    let mut integer = bytes.to_vec();

    // if the most significant bit is set, prepend a 0x00 byte
    if integer[0] & 0x80 != 0 {
        integer.insert(0, 0x00);
    }

    let mut result = vec![0x02, integer.len() as u8];
    result.extend_from_slice(&integer);

    result
}

/// Build the scriptSig from the DER signature and the public key
pub fn build_script_sig(der_signature: &[u8], public_key_bytes: &[u8]) -> Vec<u8> {
    let mut script_sig = vec![];
    script_sig.push(der_signature.len() as u8);
    script_sig.extend_from_slice(der_signature);

    script_sig.push(public_key_bytes.len() as u8);
    script_sig.extend_from_slice(public_key_bytes);

    script_sig
}

/// Serialize the ECDSA signature from the raw bytes and the SIGHASH type
pub fn serialize_ecdsa_signature(signature_bytes: &[u8], sighash_type: u8) -> Vec<u8> {
    // 1. Encode the signature as DER format
    let mut der_signature = encode_signature_as_der(signature_bytes);

    // 2. Append the SIGHASH type
    der_signature.push(sighash_type);

    der_signature
}

/// Serialize the ECDSA signature from string representations of big R and S
pub fn serialize_ecdsa_signature_from_str(big_r: &str, s: &str) -> Vec<u8> {
    // Generate the signature bytes from the hex strings
    let big_r_bytes = hex::decode(big_r).unwrap();
    let s_bytes = hex::decode(s).unwrap();

    // Create the signature bytes without the compression byte (0x02 or 0x03)
    let mut signature_bytes = vec![];
    signature_bytes.extend_from_slice(&big_r_bytes[1..]);
    signature_bytes.extend_from_slice(&s_bytes);

    // Serialize the signature using the custom function
    serialize_ecdsa_signature(&signature_bytes, 0x01) // 0x01 = SIGHASH_ALL
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitcoin::secp256k1::ecdsa::Signature;
    use bitcoin::secp256k1::{self};

    #[test]
    fn test_create_signature() {
        let big_r = "03B96BFA3DA6BB4BB74EEEE9C20970725C5782F07724CD1BEFBD265C5AD5C63948";
        let s = "49283B618968DEFB0E660EA703D193BC1D213F5DD811A2D13307FCA01E20C5C0";

        let signature_built = create_signature(big_r, s);

        let signature = bitcoin::ecdsa::Signature {
            signature: signature_built.unwrap(),
            sighash_type: bitcoin::EcdsaSighashType::All,
        };

        let serialized_with_bitcoin = signature.serialize();
        let serialized_with_bitcoin_bytes = serialized_with_bitcoin.to_vec();

        let serialized_with_custom_function = serialize_ecdsa_signature_from_str(big_r, s);

        assert_eq!(
            serialized_with_custom_function,
            serialized_with_bitcoin_bytes
        );
    }

    // using the bitcoin crate
    pub fn create_signature(big_r_hex: &str, s_hex: &str) -> Result<Signature, secp256k1::Error> {
        // Convert hex strings to byte arrays
        let big_r_bytes = hex::decode(big_r_hex).unwrap();
        let s_bytes = hex::decode(s_hex).unwrap();

        // Remove the first byte from big_r (compressed point indicator)
        let big_r_x_bytes = &big_r_bytes[1..];

        // Ensure the byte arrays are the correct length
        if big_r_x_bytes.len() != 32 || s_bytes.len() != 32 {
            return Err(secp256k1::Error::InvalidSignature);
        }

        // Create the signature from the bytes
        let mut signature_bytes = [0u8; 64];
        signature_bytes[..32].copy_from_slice(big_r_x_bytes);
        signature_bytes[32..].copy_from_slice(&s_bytes);

        // Create the signature object
        let signature = Signature::from_compact(&signature_bytes)?;

        Ok(signature)
    }
}
