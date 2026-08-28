//! TON account addresses: raw `workchain:hex` and user-friendly base64
//! forms.
use core::fmt;
use core::str::FromStr;

use base64::engine::general_purpose::{STANDARD_NO_PAD, URL_SAFE_NO_PAD};
use base64::Engine;

use crate::ton::utils::crc16_xmodem;

/// Tag byte of a bounceable user-friendly address.
const TAG_BOUNCEABLE: u8 = 0x11;
/// Tag byte of a non-bounceable user-friendly address.
const TAG_NON_BOUNCEABLE: u8 = 0x51;
/// Flag OR-ed into the tag byte for testnet-only addresses.
const FLAG_TESTNET: u8 = 0x80;

/// A TON account address: a workchain (0 = basechain, -1 = masterchain) and
/// a 256-bit account identifier (for wallets, the hash of the initial
/// `StateInit`).
///
/// Parses from both the raw form (`"0:83dfd552..."`) and the user-friendly
/// 36-byte base64 form (standard or URL-safe alphabet, bounceable or not,
/// with CRC-16/XMODEM validation). [`fmt::Display`] renders the bounceable
/// URL-safe base64 form.
///
/// The bounceable/testnet tag bits are validated but **not** stored: they
/// are display hints, not part of the account identity. Choose bounce
/// behavior per transfer on [`crate::ton::types::InternalMessage`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TonAddress {
    /// The workchain identifier (in practice 0 or -1).
    pub workchain: i8,
    /// The 32-byte account identifier within the workchain.
    pub hash: [u8; 32],
}

impl TonAddress {
    /// Creates an address from a workchain and a 32-byte account identifier.
    pub const fn new(workchain: i8, hash: [u8; 32]) -> Self {
        Self { workchain, hash }
    }

    /// Parses the raw form `"<workchain>:<64 hex digits>"`.
    ///
    /// # Errors
    ///
    /// Returns a [`TonAddressParseError`] if the workchain or hash is
    /// malformed.
    pub fn from_raw_str(s: &str) -> Result<Self, TonAddressParseError> {
        let (wc, hash_hex) = s.split_once(':').ok_or(TonAddressParseError::BadFormat)?;
        let workchain: i32 = wc.parse().map_err(|_| TonAddressParseError::BadWorkchain)?;
        let workchain = i8::try_from(workchain).map_err(|_| TonAddressParseError::BadWorkchain)?;
        if hash_hex.len() != 64 {
            return Err(TonAddressParseError::BadFormat);
        }
        let mut hash = [0u8; 32];
        hex::decode_to_slice(hash_hex, &mut hash).map_err(|_| TonAddressParseError::BadFormat)?;
        Ok(Self { workchain, hash })
    }

    /// Parses the user-friendly form: 36 bytes (tag, workchain, 32-byte
    /// hash, CRC-16/XMODEM big-endian) in base64, standard or URL-safe
    /// alphabet.
    ///
    /// # Errors
    ///
    /// Returns a [`TonAddressParseError`] if the encoding, length, tag or
    /// checksum is invalid.
    pub fn from_base64(s: &str) -> Result<Self, TonAddressParseError> {
        let bytes = URL_SAFE_NO_PAD
            .decode(s)
            .or_else(|_| STANDARD_NO_PAD.decode(s))
            .map_err(|_| TonAddressParseError::BadFormat)?;
        let bytes: [u8; 36] = bytes
            .try_into()
            .map_err(|_| TonAddressParseError::BadLength)?;
        let tag = bytes[0] & !FLAG_TESTNET;
        if tag != TAG_BOUNCEABLE && tag != TAG_NON_BOUNCEABLE {
            return Err(TonAddressParseError::BadTag);
        }
        let expected = u16::from_be_bytes([bytes[34], bytes[35]]);
        if crc16_xmodem(&bytes[..34]) != expected {
            return Err(TonAddressParseError::BadChecksum);
        }
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&bytes[2..34]);
        Ok(Self {
            workchain: bytes[1] as i8,
            hash,
        })
    }

    /// Renders the raw form `"<workchain>:<64 hex digits>"`.
    pub fn to_raw_string(&self) -> String {
        format!("{}:{}", self.workchain, hex::encode(self.hash))
    }

    /// Renders the user-friendly URL-safe base64 form with the requested
    /// bounceable and testnet-only flags.
    pub fn to_base64_string(&self, bounceable: bool, testnet: bool) -> String {
        let tag = if bounceable {
            TAG_BOUNCEABLE
        } else {
            TAG_NON_BOUNCEABLE
        } | if testnet { FLAG_TESTNET } else { 0 };
        let mut bytes = [0u8; 36];
        bytes[0] = tag;
        bytes[1] = self.workchain as u8;
        bytes[2..34].copy_from_slice(&self.hash);
        let crc = crc16_xmodem(&bytes[..34]);
        bytes[34..36].copy_from_slice(&crc.to_be_bytes());
        URL_SAFE_NO_PAD.encode(bytes)
    }
}

impl fmt::Display for TonAddress {
    /// Renders the bounceable, mainnet, URL-safe base64 form.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_base64_string(true, false))
    }
}

impl FromStr for TonAddress {
    type Err = TonAddressParseError;

    /// Accepts the raw `"workchain:hex"` form and the user-friendly base64
    /// form (either alphabet, any flags).
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.contains(':') {
            Self::from_raw_str(s)
        } else {
            Self::from_base64(s)
        }
    }
}

/// Error returned when parsing a [`TonAddress`] fails.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TonAddressParseError {
    /// Not a recognizable raw or base64 address.
    BadFormat,
    /// The decoded user-friendly form is not exactly 36 bytes.
    BadLength,
    /// The tag byte is neither bounceable (0x11) nor non-bounceable (0x51).
    BadTag,
    /// The CRC-16/XMODEM checksum does not match.
    BadChecksum,
    /// The workchain is outside the supported `i8` range.
    BadWorkchain,
}

impl fmt::Display for TonAddressParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadFormat => write!(f, "invalid TON address encoding"),
            Self::BadLength => write!(f, "user-friendly TON address must decode to 36 bytes"),
            Self::BadTag => write!(f, "invalid TON address tag byte"),
            Self::BadChecksum => write!(f, "TON address checksum mismatch"),
            Self::BadWorkchain => write!(f, "TON address workchain out of range"),
        }
    }
}

impl std::error::Error for TonAddressParseError {}

// The JSON form of an address is its user-friendly string, not the raw
// struct fields, hence the manual impls.
#[cfg(feature = "serde")]
impl serde::Serialize for TonAddress {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for TonAddress {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let text = String::deserialize(deserializer)?;
        text.parse().map_err(serde::de::Error::custom)
    }
}

// The schema mirrors the serde form (string), hence the manual impl.
#[cfg(feature = "schemars")]
impl schemars::JsonSchema for TonAddress {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "TonAddress".into()
    }

    fn json_schema(generator: &mut schemars::generate::SchemaGenerator) -> schemars::Schema {
        <String>::json_schema(generator)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RAW: &str = "0:83dfd552e63729b472fcbcc8c45ebcc6691702558b68ec7527e1ba403a0f31a8";
    const BOUNCEABLE: &str = "EQCD39VS5jcptHL8vMjEXrzGaRcCVYto7HUn4bpAOg8xqB2N";
    const NON_BOUNCEABLE: &str = "UQCD39VS5jcptHL8vMjEXrzGaRcCVYto7HUn4bpAOg8xqEBI";

    /// Spec vector: raw <-> user-friendly round trip, both flags.
    #[test]
    fn test_round_trip_matches_vectors() {
        let address = TonAddress::from_raw_str(RAW).unwrap();
        assert_eq!(address.workchain, 0);
        assert_eq!(address.to_base64_string(true, false), BOUNCEABLE);
        assert_eq!(address.to_base64_string(false, false), NON_BOUNCEABLE);
        assert_eq!(address.to_string(), BOUNCEABLE);
        assert_eq!(address.to_raw_string(), RAW);

        assert_eq!(TonAddress::from_base64(BOUNCEABLE).unwrap(), address);
        assert_eq!(TonAddress::from_base64(NON_BOUNCEABLE).unwrap(), address);
        assert_eq!(RAW.parse::<TonAddress>().unwrap(), address);
        assert_eq!(BOUNCEABLE.parse::<TonAddress>().unwrap(), address);
    }

    #[test]
    fn test_accepts_standard_base64_alphabet() {
        let address = TonAddress::from_raw_str(RAW).unwrap();
        let url_safe = address.to_base64_string(true, false);
        let standard = url_safe.replace('-', "+").replace('_', "/");
        assert_eq!(TonAddress::from_base64(&standard).unwrap(), address);
    }

    #[test]
    fn test_testnet_flag_round_trip() {
        let address = TonAddress::from_raw_str(RAW).unwrap();
        let testnet = address.to_base64_string(true, true);
        assert_ne!(testnet, BOUNCEABLE);
        assert_eq!(TonAddress::from_base64(&testnet).unwrap(), address);
    }

    #[test]
    fn test_masterchain_round_trip() {
        let raw = "-1:83dfd552e63729b472fcbcc8c45ebcc6691702558b68ec7527e1ba403a0f31a8";
        let address = TonAddress::from_raw_str(raw).unwrap();
        assert_eq!(address.workchain, -1);
        let friendly = address.to_base64_string(true, false);
        assert_eq!(TonAddress::from_base64(&friendly).unwrap(), address);
        assert_eq!(address.to_raw_string(), raw);
    }

    #[test]
    fn test_rejects_invalid_input() {
        // Corrupt the checksum.
        let mut broken = String::from(BOUNCEABLE);
        broken.replace_range(46..48, "AA");
        assert_eq!(
            TonAddress::from_base64(&broken).unwrap_err(),
            TonAddressParseError::BadChecksum
        );
        // Bad tag byte (0x00 leading byte re-checksummed).
        let mut bytes = [0u8; 36];
        let crc = crc16_xmodem(&bytes[..34]);
        bytes[34..36].copy_from_slice(&crc.to_be_bytes());
        let encoded = URL_SAFE_NO_PAD.encode(bytes);
        assert_eq!(
            TonAddress::from_base64(&encoded).unwrap_err(),
            TonAddressParseError::BadTag
        );
        // Bad lengths and encodings.
        assert!(TonAddress::from_base64("AAAA").is_err());
        assert!(TonAddress::from_base64("!!!").is_err());
        assert!(TonAddress::from_raw_str("0:1234").is_err());
        assert!(TonAddress::from_raw_str("300:1234").is_err());
        assert!("".parse::<TonAddress>().is_err());
    }

    #[cfg(feature = "serde_json")]
    #[test]
    fn test_serde_uses_friendly_string() {
        let address = TonAddress::from_raw_str(RAW).unwrap();
        let json = serde_json::to_string(&address).unwrap();
        assert_eq!(json, format!("\"{BOUNCEABLE}\""));
        let back: TonAddress = serde_json::from_str(&json).unwrap();
        assert_eq!(back, address);
        // Raw form is accepted on input too.
        let from_raw: TonAddress = serde_json::from_str(&format!("\"{RAW}\"")).unwrap();
        assert_eq!(from_raw, address);
    }
}
