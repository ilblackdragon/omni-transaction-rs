//! Move identifier and module id types.
use core::fmt;

use super::address::AccountAddress;
use crate::bcs_encoding::{BcsEncode, BcsWriter};
#[cfg(feature = "schemars")]
use schemars::JsonSchema;
#[cfg(feature = "serde")]
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Maximum byte length of an identifier that can appear in an Aptos module.
///
/// `move-core-types`' `identifier::is_valid` (the shared Move origin) only
/// checks the charset and imposes no length limit; the length is bounded by
/// the module binary format instead. Aptos' `move-binary-format`
/// `file_format_common::IDENTIFIER_SIZE_MAX` is 255 and the on-chain VM
/// deserializer uses it whenever the `LIMIT_MAX_IDENTIFIER_LENGTH` feature
/// flag is on (mainnet since aptos-node v1.8; before it the legacy limit was
/// `LEGACY_IDENTIFIER_SIZE_MAX` = 65535). No loadable Aptos module can
/// therefore expose a module, function, struct or field name longer than
/// this, so a longer identifier can never name a callable target.
///
/// Note this differs from Sui, whose protocol config caps
/// `max_move_identifier_len` at 128 — see
/// `omni_transaction::sui::types::MAX_IDENTIFIER_LENGTH`. The two limits are
/// per-chain and deliberately not shared.
pub const MAX_IDENTIFIER_LENGTH: usize = 255;

/// A valid Move identifier (module or function name).
///
/// Identifiers match `[a-zA-Z][a-zA-Z0-9_]*` or `_[a-zA-Z0-9_]+` (a lone `_`
/// is not a valid identifier), mirroring `move-core-types`'
/// `identifier::is_valid`, and are at most
/// [`MAX_IDENTIFIER_LENGTH`] bytes long (Aptos' module binary format limit).
///
/// In BCS an identifier is a string: ULEB128 byte length followed by the
/// UTF-8 bytes. The JSON (serde) representation is a plain string.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Identifier(String);

impl Identifier {
    /// Creates an identifier, validating the Move identifier charset and the
    /// [`MAX_IDENTIFIER_LENGTH`] byte cap.
    ///
    /// # Errors
    ///
    /// Returns [`IdentifierParseError`] if the string is not a valid Move
    /// identifier.
    pub fn new(name: impl Into<String>) -> Result<Self, IdentifierParseError> {
        let name = name.into();
        if is_valid_identifier(&name) {
            Ok(Self(name))
        } else {
            Err(IdentifierParseError)
        }
    }

    /// Returns the identifier as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consumes the identifier, returning the underlying string.
    pub fn into_string(self) -> String {
        self.0
    }
}

/// Returns `true` if `name` matches `[a-zA-Z][a-zA-Z0-9_]*` or
/// `_[a-zA-Z0-9_]+` and is at most [`MAX_IDENTIFIER_LENGTH`] bytes long.
fn is_valid_identifier(name: &str) -> bool {
    let bytes = name.as_bytes();
    if bytes.len() > MAX_IDENTIFIER_LENGTH {
        return false;
    }
    let rest = match bytes.first() {
        Some(b'a'..=b'z' | b'A'..=b'Z') => &bytes[1..],
        Some(b'_') if bytes.len() > 1 => &bytes[1..],
        _ => return false,
    };
    rest.iter().all(|b| b.is_ascii_alphanumeric() || *b == b'_')
}

impl fmt::Display for Identifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl core::str::FromStr for Identifier {
    type Err = IdentifierParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl From<Identifier> for String {
    fn from(identifier: Identifier) -> Self {
        identifier.0
    }
}

impl BcsEncode for Identifier {
    fn bcs_encode(&self, writer: &mut BcsWriter) {
        writer.write_string(&self.0);
    }
}

/// Error returned when a string is not a valid Move identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IdentifierParseError;

impl fmt::Display for IdentifierParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "invalid Move identifier: expected [a-zA-Z][a-zA-Z0-9_]* or _[a-zA-Z0-9_]+ of at most 255 bytes"
        )
    }
}

impl std::error::Error for IdentifierParseError {}

// Serde is implemented by hand so that deserialization enforces the Move
// identifier invariant (a derive on the private field would bypass `new`).
#[cfg(feature = "serde")]
impl Serialize for Identifier {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for Identifier {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let name = String::deserialize(deserializer)?;
        Self::new(name).map_err(serde::de::Error::custom)
    }
}

// The schema mirrors the serde form (a plain string).
#[cfg(feature = "schemars")]
impl JsonSchema for Identifier {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "Identifier".into()
    }

    fn json_schema(generator: &mut schemars::generate::SchemaGenerator) -> schemars::Schema {
        <String>::json_schema(generator)
    }
}

/// The id of a Move module: the address that published it plus its name.
///
/// In BCS: 32 raw address bytes followed by the name as a ULEB128
/// length-prefixed string (see `move-core-types`' `language_storage.rs`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct ModuleId {
    /// The address that published the module.
    pub address: AccountAddress,
    /// The module name.
    pub name: Identifier,
}

impl ModuleId {
    /// Creates a module id from an address and a module name.
    pub const fn new(address: AccountAddress, name: Identifier) -> Self {
        Self { address, name }
    }
}

impl fmt::Display for ModuleId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}::{}", self.address, self.name)
    }
}

impl BcsEncode for ModuleId {
    fn bcs_encode(&self, writer: &mut BcsWriter) {
        self.address.bcs_encode(writer);
        self.name.bcs_encode(writer);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bcs_encoding::to_bytes;

    #[test]
    fn test_identifier_validation() {
        assert!(Identifier::new("transfer").is_ok());
        assert!(Identifier::new("aptos_coin").is_ok());
        assert!(Identifier::new("_private").is_ok());
        assert!(Identifier::new("v2").is_ok());
        assert!(Identifier::new("").is_err());
        assert!(Identifier::new("_").is_err());
        assert!(Identifier::new("1abc").is_err());
        assert!(Identifier::new("has-dash").is_err());
        assert!(Identifier::new("has space").is_err());
        assert!(Identifier::new("emoji🚀").is_err());
    }

    /// Aptos bounds identifiers by its module binary format, not by
    /// `move-core-types`' charset check: `IDENTIFIER_SIZE_MAX` = 255 bytes
    /// (the pre-`LIMIT_MAX_IDENTIFIER_LENGTH` legacy limit was 65535). Sui's
    /// own limit is 128 and lives in its own module.
    #[test]
    fn test_identifier_length_limit_is_aptos_255_bytes() {
        assert_eq!(MAX_IDENTIFIER_LENGTH, 255);
        assert!(Identifier::new("a".repeat(255)).is_ok());
        // Longer than any Aptos module can expose, so it can never resolve.
        assert!(Identifier::new("a".repeat(256)).is_err());
        // Above Sui's 128-byte cap but valid here: the limits are per-chain.
        assert!(Identifier::new("a".repeat(129)).is_ok());
    }

    #[test]
    fn test_identifier_bcs_matches_reference_bcs_string() {
        let identifier = Identifier::new("aptos_coin").unwrap();
        let encoded = to_bytes(&identifier);
        assert_eq!(encoded, bcs::to_bytes(&"aptos_coin").unwrap());
        assert_eq!(hex::encode(&encoded), "0a6170746f735f636f696e");
    }

    #[test]
    fn test_module_id_bcs_is_address_then_name() {
        let module_id = ModuleId::new(
            AccountAddress::from_hex("0x1222").unwrap(),
            Identifier::new("aptos_coin").unwrap(),
        );
        let encoded = to_bytes(&module_id);
        assert_eq!(
            hex::encode(encoded),
            "00000000000000000000000000000000000000000000000000000000000012220a6170746f735f636f696e"
        );
    }

    #[test]
    #[cfg(feature = "serde_json")]
    fn test_identifier_serde_round_trip_enforces_validation() {
        let identifier = Identifier::new("transfer").unwrap();
        let json = serde_json::to_string(&identifier).unwrap();
        assert_eq!(json, "\"transfer\"");
        let back: Identifier = serde_json::from_str(&json).unwrap();
        assert_eq!(back, identifier);

        let invalid: Result<Identifier, _> = serde_json::from_str("\"not valid\"");
        assert!(invalid.is_err());
    }
}
