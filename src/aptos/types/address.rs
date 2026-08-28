//! Aptos account address type.
use core::fmt;

use crate::bcs_encoding::{BcsEncode, BcsWriter};
#[cfg(feature = "schemars")]
use schemars::JsonSchema;
#[cfg(feature = "serde")]
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Byte length of an [`AccountAddress`].
pub const ACCOUNT_ADDRESS_LENGTH: usize = 32;

/// A 32-byte Aptos account address.
///
/// In BCS this is a fixed-size array: 32 raw bytes with **no** length prefix
/// (see `move-core-types`' `AccountAddress`).
///
/// The JSON (serde) representation is a `0x`-prefixed lowercase hex string;
/// short forms such as `"0x1"` are accepted on input and left-padded with
/// zeros.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AccountAddress(pub [u8; ACCOUNT_ADDRESS_LENGTH]);

impl AccountAddress {
    /// The all-zeros address (`0x0`).
    pub const ZERO: Self = Self([0u8; ACCOUNT_ADDRESS_LENGTH]);

    /// The Aptos framework address (`0x1`).
    pub const ONE: Self = {
        let mut bytes = [0u8; ACCOUNT_ADDRESS_LENGTH];
        bytes[ACCOUNT_ADDRESS_LENGTH - 1] = 1;
        Self(bytes)
    };

    /// Creates an address from raw bytes.
    pub const fn new(bytes: [u8; ACCOUNT_ADDRESS_LENGTH]) -> Self {
        Self(bytes)
    }

    /// Returns the raw 32-byte representation.
    pub const fn into_inner(self) -> [u8; ACCOUNT_ADDRESS_LENGTH] {
        self.0
    }

    /// Returns the address bytes as a slice.
    pub const fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// Parses an address from a hex string, with or without a `0x` prefix.
    ///
    /// Inputs shorter than 64 hex characters are left-padded with zeros, so
    /// canonical short forms such as `"0x1"` parse to `0x0000...0001`
    /// (matching `AccountAddress::from_hex_literal` in `move-core-types`).
    ///
    /// # Errors
    ///
    /// Returns [`AddressParseError`] if the input is empty, longer than 64
    /// hex characters or contains non-hex characters.
    pub fn from_hex(hex_str: &str) -> Result<Self, AddressParseError> {
        let hex_str = hex_str.strip_prefix("0x").unwrap_or(hex_str);
        if hex_str.is_empty() || hex_str.len() > ACCOUNT_ADDRESS_LENGTH * 2 {
            return Err(AddressParseError);
        }
        let mut padded = [b'0'; ACCOUNT_ADDRESS_LENGTH * 2];
        padded[ACCOUNT_ADDRESS_LENGTH * 2 - hex_str.len()..].copy_from_slice(hex_str.as_bytes());
        let mut bytes = [0u8; ACCOUNT_ADDRESS_LENGTH];
        hex::decode_to_slice(padded, &mut bytes).map_err(|_| AddressParseError)?;
        Ok(Self(bytes))
    }

    /// Renders the address as a `0x`-prefixed, zero-padded hex string.
    pub fn to_hex(&self) -> String {
        format!("0x{}", hex::encode(self.0))
    }
}

impl fmt::Display for AccountAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

impl From<[u8; ACCOUNT_ADDRESS_LENGTH]> for AccountAddress {
    fn from(bytes: [u8; ACCOUNT_ADDRESS_LENGTH]) -> Self {
        Self(bytes)
    }
}

impl core::str::FromStr for AccountAddress {
    type Err = AddressParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_hex(s)
    }
}

impl BcsEncode for AccountAddress {
    fn bcs_encode(&self, writer: &mut BcsWriter) {
        writer.write_fixed(&self.0);
    }
}

/// Error returned when parsing an [`AccountAddress`] from a hex string fails.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AddressParseError;

impl fmt::Display for AddressParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid Aptos address: expected up to 64 hex characters")
    }
}

impl std::error::Error for AddressParseError {}

// The JSON form of an address is a hex string, not the raw byte array, so
// serde is implemented by hand (the wire format is the hand-rolled BCS above
// and is never derived from serde).
#[cfg(feature = "serde")]
impl Serialize for AccountAddress {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_hex())
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for AccountAddress {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct AddressVisitor;

        impl<'de> serde::de::Visitor<'de> for AddressVisitor {
            type Value = AccountAddress;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a hex string or an array of 32 bytes")
            }

            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
                AccountAddress::from_hex(v).map_err(E::custom)
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let mut bytes = [0u8; ACCOUNT_ADDRESS_LENGTH];
                for (i, byte) in bytes.iter_mut().enumerate() {
                    *byte = seq
                        .next_element::<u8>()?
                        .ok_or_else(|| serde::de::Error::invalid_length(i, &"32 bytes"))?;
                }
                if seq.next_element::<u8>()?.is_some() {
                    return Err(serde::de::Error::custom("expected exactly 32 bytes"));
                }
                Ok(AccountAddress(bytes))
            }
        }

        deserializer.deserialize_any(AddressVisitor)
    }
}

// The schema mirrors the serde form (hex string), which the derive on
// `[u8; 32]` would not, hence the manual impl.
#[cfg(feature = "schemars")]
impl JsonSchema for AccountAddress {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "AccountAddress".into()
    }

    fn json_schema(generator: &mut schemars::generate::SchemaGenerator) -> schemars::Schema {
        <String>::json_schema(generator)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bcs_encoding::to_bytes;

    #[test]
    fn test_from_hex_pads_short_addresses() {
        let address = AccountAddress::from_hex("0x1").unwrap();
        assert_eq!(address, AccountAddress::ONE);
        assert_eq!(
            address.to_string(),
            "0x0000000000000000000000000000000000000000000000000000000000000001"
        );
    }

    #[test]
    fn test_from_hex_accepts_full_length_with_and_without_prefix() {
        let full = "1111111111111111111111111111111111111111111111111111111111111111";
        assert_eq!(
            AccountAddress::from_hex(full).unwrap(),
            AccountAddress::new([0x11u8; 32])
        );
        assert_eq!(
            AccountAddress::from_hex(&format!("0x{full}")).unwrap(),
            AccountAddress::new([0x11u8; 32])
        );
    }

    #[test]
    fn test_from_hex_rejects_invalid_input() {
        assert!(AccountAddress::from_hex("").is_err());
        assert!(AccountAddress::from_hex("0x").is_err());
        assert!(AccountAddress::from_hex("zz").is_err());
        let too_long = "11".repeat(33);
        assert!(AccountAddress::from_hex(&too_long).is_err());
    }

    #[test]
    fn test_bcs_encoding_is_fixed_32_bytes_without_prefix() {
        let address = AccountAddress::new([0xabu8; 32]);
        let encoded = to_bytes(&address);
        assert_eq!(encoded.len(), 32);
        assert_eq!(encoded, vec![0xabu8; 32]);
        // Cross-check against the reference bcs crate ([u8; 32] is BCS-encoded
        // as 32 raw bytes, exactly like aptos-core's AccountAddress).
        assert_eq!(encoded, bcs::to_bytes(&[0xabu8; 32]).unwrap());
    }

    #[test]
    #[cfg(feature = "serde_json")]
    fn test_serde_hex_string_and_byte_array_round_trip() {
        let address = AccountAddress::from_hex("0xa550c18").unwrap();
        let json = serde_json::to_string(&address).unwrap();
        assert_eq!(
            json,
            "\"0x000000000000000000000000000000000000000000000000000000000a550c18\""
        );
        let back: AccountAddress = serde_json::from_str(&json).unwrap();
        assert_eq!(back, address);

        let array = serde_json::to_string(&address.into_inner().to_vec()).unwrap();
        let from_array: AccountAddress = serde_json::from_str(&array).unwrap();
        assert_eq!(from_array, address);
    }
}
