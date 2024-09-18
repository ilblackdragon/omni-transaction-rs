use std::io::{self, BufRead, Write};

use borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::serde::{Deserialize, Serialize};

use crate::bitcoin::encoding::{Decodable, Encodable};

/// The transaction version.
///
/// Currently, as specified by [BIP-68], only version 1 and 2 are considered standard.
///
/// [BIP-68]: https://github.com/bitcoin/bips/blob/master/bip-0068.mediawiki
#[derive(
    Debug, Copy, PartialEq, Eq, Clone, Serialize, Deserialize, BorshSerialize, BorshDeserialize,
)]
#[borsh(use_discriminant = true)]
pub enum Version {
    /// The original Bitcoin transaction version (pre-BIP-68)
    One = 1,
    /// The second Bitcoin transaction version (post-BIP-68)
    Two = 2,
}

impl Version {
    /// Returns the hexadecimal representation of the version.
    pub fn to_hex(&self) -> String {
        hex::encode(&(*self as i32).to_le_bytes())
    }

    /// Serializes the version and returns the result as a Vec<u8>.
    pub fn to_vec(&self) -> Vec<u8> {
        (*self as i32).to_le_bytes().to_vec()
    }
}

impl Encodable for Version {
    fn encode<W: Write + ?Sized>(&self, w: &mut W) -> Result<usize, io::Error> {
        let bytes = (*self as i32).to_le_bytes();
        w.write_all(&bytes)?;
        Ok(bytes.len())
    }
}

impl Decodable for Version {
    fn decode<R: BufRead + ?Sized>(r: &mut R) -> Result<Self, std::io::Error> {
        let mut buf = [0u8; 4];
        r.read_exact(&mut buf)?;
        let int = i32::from_le_bytes(buf);

        match int {
            1 => Ok(Version::One),
            2 => Ok(Version::Two),
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid version number",
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_version_serialization() {
        let version = Version::One;
        let mut buf = Vec::new();

        version.encode(&mut buf).unwrap();

        // Check that the serialized bytes are correct
        assert_eq!(buf, vec![1, 0, 0, 0]);

        // Check the hexadecimal representation
        assert_eq!(version.to_hex(), "01000000");
    }

    #[test]
    fn test_version_deserialization() {
        let data = vec![1, 0, 0, 0];
        let mut cursor = Cursor::new(data);
        let version = Version::decode(&mut cursor).unwrap();

        // Check that the deserialized version is correct
        assert_eq!(version, Version::One);
    }

    #[test]
    fn test_version_round_trip() {
        let version = Version::Two;
        let mut buf = Vec::new();
        version.encode(&mut buf).unwrap();
        let mut cursor = Cursor::new(buf);
        let decoded_version = Version::decode(&mut cursor).unwrap();

        // Check that the version is the same after encoding and decoding
        assert_eq!(version, decoded_version);
    }

    #[test]
    fn test_version_to_vec() {
        let version = Version::One;
        let vec = version.to_vec();

        // Check that the serialized bytes are correct
        assert_eq!(vec, vec![1, 0, 0, 0]);
    }

    #[test]
    fn test_version_to_hex() {
        let version = Version::One;
        let hex = version.to_hex();

        // Check that the hexadecimal representation is correct
        assert_eq!(hex, "01000000");
    }

    #[test]
    fn test_version_serde_serialization() {
        let version = Version::One;
        let serialized = serde_json::to_string(&version).unwrap();
        let deserialized: Version = serde_json::from_str(&serialized).unwrap();

        // Check that the version is the same after serde serialization and deserialization
        assert_eq!(version, deserialized);
    }

    #[test]
    fn test_version_borsh_serialization() {
        let version = Version::One;
        let buf = borsh::to_vec(&version).unwrap();
        let deserialized = Version::try_from_slice(&buf).unwrap();

        assert_eq!(version, deserialized);
    }
}
