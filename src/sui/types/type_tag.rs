//! Move type tags and identifiers used by Sui `MoveCall` commands.
use core::fmt;

use super::address::SuiAddress;
use crate::bcs_encoding::{BcsEncode, BcsWriter};
#[cfg(feature = "schemars")]
use schemars::JsonSchema;
#[cfg(feature = "serde")]
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Maximum byte length of a Move identifier on Sui.
///
/// `move-core-types`' `identifier::is_valid` (the shared Move origin) checks
/// only the charset; the length is capped by Sui's own protocol config field
/// `max_move_identifier_len`, which is 128 on mainnet (it was unset — i.e.
/// unlimited — before protocol version 9). No published Sui module can expose
/// a longer name, so a longer identifier can never name a callable target.
///
/// This is a per-chain limit: Aptos bounds identifiers at 255 bytes instead
/// (see `omni_transaction::aptos::types::MAX_IDENTIFIER_LENGTH`).
pub const MAX_IDENTIFIER_LENGTH: usize = 128;

/// A validated Move identifier (module or function name).
///
/// Valid identifiers are 1..=128 bytes, start with a letter or underscore
/// (a lone underscore is not allowed) and contain only ASCII letters, digits
/// and underscores.
///
/// # BCS
///
/// A ULEB128 length prefix followed by the UTF-8 bytes.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Identifier(String);

impl Identifier {
    /// Creates a validated identifier.
    ///
    /// # Errors
    ///
    /// Returns [`IdentifierParseError`] if the input is not a valid Move
    /// identifier.
    pub fn new<T: Into<String>>(identifier: T) -> Result<Self, IdentifierParseError> {
        let identifier = identifier.into();
        if Self::is_valid(&identifier) {
            Ok(Self(identifier))
        } else {
            Err(IdentifierParseError)
        }
    }

    /// Returns the identifier as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn is_valid(identifier: &str) -> bool {
        if identifier.is_empty() || identifier.len() > MAX_IDENTIFIER_LENGTH || identifier == "_" {
            return false;
        }
        let mut chars = identifier.chars();
        let first = chars.next().expect("non-empty checked above");
        (first.is_ascii_alphabetic() || first == '_')
            && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
    }
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
            "invalid Move identifier: expected [a-zA-Z_][a-zA-Z0-9_]* of 1..=128 bytes"
        )
    }
}

impl std::error::Error for IdentifierParseError {}

// Serialized as a plain string; deserialization re-validates.
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
        let s = String::deserialize(deserializer)?;
        Self::new(s).map_err(serde::de::Error::custom)
    }
}

#[cfg(feature = "schemars")]
impl JsonSchema for Identifier {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "Identifier".into()
    }

    fn json_schema(generator: &mut schemars::generate::SchemaGenerator) -> schemars::Schema {
        <String>::json_schema(generator)
    }
}

/// The type of a Move value.
///
/// Note: the Aptos module's `TypeTag` declares an identical table today,
/// but the two are deliberately **not** shared — each index table is frozen
/// by its own chain's fork of `move-core-types`, and the forks already
/// diverge (different reserved indices upstream, different identifier length
/// limits). Sharing one type would let a variant added for one chain silently
/// change the other chain's wire format.
///
/// # BCS
///
/// A ULEB128 variant index followed by the payload. The indices are
/// **historical, not ordinal** — `u16`/`u32`/`u256` were appended after
/// `struct`:
///
/// | variant | index |
/// |---------|-------|
/// | `bool` | `0x00` |
/// | `u8` | `0x01` |
/// | `u64` | `0x02` |
/// | `u128` | `0x03` |
/// | `address` | `0x04` |
/// | `signer` | `0x05` |
/// | `vector<T>` | `0x06` + inner tag |
/// | `struct` | `0x07` + [`StructTag`] |
/// | `u16` | `0x08` |
/// | `u32` | `0x09` |
/// | `u256` | `0x0a` |
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub enum TypeTag {
    /// `bool`
    Bool,
    /// `u8`
    U8,
    /// `u64`
    U64,
    /// `u128`
    U128,
    /// `address`
    Address,
    /// `signer`
    Signer,
    /// `vector<T>`
    Vector(Box<Self>),
    /// A struct type such as `0x2::sui::SUI`
    Struct(Box<StructTag>),
    /// `u16`
    U16,
    /// `u32`
    U32,
    /// `u256`
    U256,
}

impl BcsEncode for TypeTag {
    fn bcs_encode(&self, writer: &mut BcsWriter) {
        match self {
            Self::Bool => writer.write_variant(0),
            Self::U8 => writer.write_variant(1),
            Self::U64 => writer.write_variant(2),
            Self::U128 => writer.write_variant(3),
            Self::Address => writer.write_variant(4),
            Self::Signer => writer.write_variant(5),
            Self::Vector(inner) => {
                writer.write_variant(6);
                inner.bcs_encode(writer);
            }
            Self::Struct(struct_tag) => {
                writer.write_variant(7);
                struct_tag.bcs_encode(writer);
            }
            Self::U16 => writer.write_variant(8),
            Self::U32 => writer.write_variant(9),
            Self::U256 => writer.write_variant(10),
        }
    }
}

/// The fully-qualified name of a Move struct, e.g. `0x2::sui::SUI`.
///
/// # BCS
///
/// `32-byte address || module identifier || name identifier ||
/// vector<TypeTag> type_params`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct StructTag {
    /// Address of the package defining the struct.
    pub address: SuiAddress,
    /// Module the struct is defined in.
    pub module: Identifier,
    /// Name of the struct.
    pub name: Identifier,
    /// Generic type parameters, if any.
    pub type_params: Vec<TypeTag>,
}

impl StructTag {
    /// The type of the native SUI coin, `0x2::sui::SUI`.
    ///
    /// # Panics
    ///
    /// Never panics: the identifiers are statically valid.
    pub fn sui() -> Self {
        Self {
            address: SuiAddress::from_hex("0x2").expect("valid address"),
            module: Identifier::new("sui").expect("valid identifier"),
            name: Identifier::new("SUI").expect("valid identifier"),
            type_params: vec![],
        }
    }
}

impl BcsEncode for StructTag {
    fn bcs_encode(&self, writer: &mut BcsWriter) {
        self.address.bcs_encode(writer);
        self.module.bcs_encode(writer);
        self.name.bcs_encode(writer);
        self.type_params.bcs_encode(writer);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bcs_encoding::to_bytes;
    use std::str::FromStr;

    fn encoded_hex(tag: &TypeTag) -> String {
        hex::encode(to_bytes(tag))
    }

    /// Spec vector: the full TypeTag index table.
    #[test]
    fn test_type_tag_variant_indices() {
        assert_eq!(encoded_hex(&TypeTag::Bool), "00");
        assert_eq!(encoded_hex(&TypeTag::U8), "01");
        assert_eq!(encoded_hex(&TypeTag::U64), "02");
        assert_eq!(encoded_hex(&TypeTag::U128), "03");
        assert_eq!(encoded_hex(&TypeTag::Address), "04");
        assert_eq!(encoded_hex(&TypeTag::Signer), "05");
        assert_eq!(encoded_hex(&TypeTag::U16), "08");
        assert_eq!(encoded_hex(&TypeTag::U32), "09");
        assert_eq!(encoded_hex(&TypeTag::U256), "0a");
        assert_eq!(encoded_hex(&TypeTag::Vector(Box::new(TypeTag::U8))), "0601");
        assert_eq!(
            encoded_hex(&TypeTag::Struct(Box::new(StructTag::sui()))),
            "070000000000000000000000000000000000000000000000000000000000000002037375690353554900"
        );
        let coin_sui = TypeTag::Struct(Box::new(StructTag {
            address: SuiAddress::from_hex("0x2").unwrap(),
            module: Identifier::new("coin").unwrap(),
            name: Identifier::new("Coin").unwrap(),
            type_params: vec![TypeTag::Struct(Box::new(StructTag::sui()))],
        }));
        assert_eq!(
            encoded_hex(&coin_sui),
            "07000000000000000000000000000000000000000000000000000000000000000204636f696e04436f696e01070000000000000000000000000000000000000000000000000000000000000002037375690353554900"
        );
    }

    /// Cross-check every variant against the reference sui-sdk-types crate.
    #[test]
    fn test_type_tag_against_reference_sdk() {
        let cases = [
            "bool",
            "u8",
            "u16",
            "u32",
            "u64",
            "u128",
            "u256",
            "address",
            "signer",
            "vector<u8>",
            "vector<vector<u8>>",
            "0x2::sui::SUI",
            "0x2::coin::Coin<0x2::sui::SUI>",
        ];
        let ours = [
            TypeTag::Bool,
            TypeTag::U8,
            TypeTag::U16,
            TypeTag::U32,
            TypeTag::U64,
            TypeTag::U128,
            TypeTag::U256,
            TypeTag::Address,
            TypeTag::Signer,
            TypeTag::Vector(Box::new(TypeTag::U8)),
            TypeTag::Vector(Box::new(TypeTag::Vector(Box::new(TypeTag::U8)))),
            TypeTag::Struct(Box::new(StructTag::sui())),
            TypeTag::Struct(Box::new(StructTag {
                address: SuiAddress::from_hex("0x2").unwrap(),
                module: Identifier::new("coin").unwrap(),
                name: Identifier::new("Coin").unwrap(),
                type_params: vec![TypeTag::Struct(Box::new(StructTag::sui()))],
            })),
        ];
        for (ours, name) in ours.iter().zip(cases.iter()) {
            let reference = sui_sdk_types::TypeTag::from_str(name).unwrap();
            assert_eq!(
                to_bytes(ours),
                ::bcs::to_bytes(&reference).unwrap(),
                "TypeTag mismatch for {name}"
            );
        }
    }

    #[test]
    fn test_identifier_validation() {
        assert!(Identifier::new("transfer").is_ok());
        assert!(Identifier::new("_private").is_ok());
        assert!(Identifier::new("v2_pool").is_ok());
        assert!(Identifier::new("A").is_ok());
        assert!(Identifier::new("a".repeat(128)).is_ok());

        assert!(Identifier::new("").is_err());
        assert!(Identifier::new("_").is_err());
        assert!(Identifier::new("2fast").is_err());
        assert!(Identifier::new("has-dash").is_err());
        assert!(Identifier::new("has space").is_err());
        assert!(Identifier::new("ünïcode").is_err());
        assert!(Identifier::new("a".repeat(129)).is_err());
    }

    /// Sui's cap is its protocol config `max_move_identifier_len` = 128,
    /// which is stricter than Aptos' 255-byte module-format limit. Pinned so
    /// the two chains' rules cannot be accidentally unified.
    #[test]
    fn test_identifier_length_limit_is_sui_128_bytes() {
        assert_eq!(MAX_IDENTIFIER_LENGTH, 128);
        assert!(Identifier::new("a".repeat(MAX_IDENTIFIER_LENGTH)).is_ok());
        assert!(Identifier::new("a".repeat(MAX_IDENTIFIER_LENGTH + 1)).is_err());
    }
}
