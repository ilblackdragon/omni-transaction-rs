//! Move type tags (`TypeTag`, `StructTag`).
use core::fmt;

use super::address::AccountAddress;
use super::identifier::Identifier;
use crate::bcs_encoding::{BcsEncode, BcsWriter};
#[cfg(feature = "schemars")]
use schemars::JsonSchema;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// A Move runtime type, used as a type argument for entry functions and
/// scripts.
///
/// **The declaration order is the wire format** — BCS encodes the ULEB128
/// variant index followed by the variant payload. The indices are frozen by
/// `move-core-types`' `language_storage.rs` (`TypeTag`):
///
/// | index | variant  |
/// |-------|----------|
/// | 0     | `Bool`   |
/// | 1     | `U8`     |
/// | 2     | `U64`    |
/// | 3     | `U128`   |
/// | 4     | `Address`|
/// | 5     | `Signer` |
/// | 6     | `Vector` |
/// | 7     | `Struct` |
/// | 8     | `U16`    |
/// | 9     | `U32`    |
/// | 10    | `U256`   |
///
/// `U16`/`U32`/`U256` were added in bytecode version v6 **after** `Struct` —
/// never reorder. Indices 11+ (`Function`, `I8`..`I256`) exist upstream and
/// are intentionally not implemented; never reuse their indices.
///
/// # Why this is not shared with the Sui module
///
/// The Sui module's `TypeTag` currently declares an identical table, but
/// the two are deliberately kept separate: each index table is frozen by its
/// own chain's fork of `move-core-types`, and the forks already diverge
/// (Aptos has reserved 11+ for `Function`/`I8`..`I256`; Sui evolves
/// independently, and enforces a different identifier length limit). Sharing
/// one type would mean that adding a variant for one chain silently changes
/// the other chain's wire format. Keep them separate and keep both index
/// tables pinned by tests.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub enum TypeTag {
    /// `bool` (index 0).
    Bool,
    /// `u8` (index 1).
    U8,
    /// `u64` (index 2).
    U64,
    /// `u128` (index 3).
    U128,
    /// `address` (index 4).
    Address,
    /// `signer` (index 5).
    Signer,
    /// `vector<T>` (index 6, recursive).
    Vector(Box<Self>),
    /// A struct type such as `0x1::aptos_coin::AptosCoin` (index 7).
    Struct(Box<StructTag>),
    /// `u16` (index 8).
    U16,
    /// `u32` (index 9).
    U32,
    /// `u256` (index 10).
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

impl fmt::Display for TypeTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bool => write!(f, "bool"),
            Self::U8 => write!(f, "u8"),
            Self::U64 => write!(f, "u64"),
            Self::U128 => write!(f, "u128"),
            Self::Address => write!(f, "address"),
            Self::Signer => write!(f, "signer"),
            Self::Vector(inner) => write!(f, "vector<{inner}>"),
            Self::Struct(struct_tag) => write!(f, "{struct_tag}"),
            Self::U16 => write!(f, "u16"),
            Self::U32 => write!(f, "u32"),
            Self::U256 => write!(f, "u256"),
        }
    }
}

/// A fully-qualified Move struct type, e.g. `0x1::aptos_coin::AptosCoin`.
///
/// In BCS: 32 raw address bytes, module [`Identifier`], struct name
/// [`Identifier`], then the type arguments as a ULEB128-counted sequence of
/// [`TypeTag`]s.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct StructTag {
    /// The address that published the module defining the struct.
    pub address: AccountAddress,
    /// The module defining the struct.
    pub module: Identifier,
    /// The struct name.
    pub name: Identifier,
    /// Generic type arguments, empty for non-generic structs.
    pub type_args: Vec<TypeTag>,
}

impl StructTag {
    /// Creates a struct tag.
    pub const fn new(
        address: AccountAddress,
        module: Identifier,
        name: Identifier,
        type_args: Vec<TypeTag>,
    ) -> Self {
        Self {
            address,
            module,
            name,
            type_args,
        }
    }

    /// The `0x1::aptos_coin::AptosCoin` struct tag, the coin type of the
    /// native APT token.
    ///
    /// # Panics
    ///
    /// Never panics: the identifiers are compile-time constants that are
    /// valid Move identifiers.
    pub fn aptos_coin() -> Self {
        Self::new(
            AccountAddress::ONE,
            Identifier::new("aptos_coin").expect("valid identifier"),
            Identifier::new("AptosCoin").expect("valid identifier"),
            vec![],
        )
    }
}

impl fmt::Display for StructTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}::{}::{}", self.address, self.module, self.name)?;
        if let Some((first, rest)) = self.type_args.split_first() {
            write!(f, "<{first}")?;
            for type_arg in rest {
                write!(f, ", {type_arg}")?;
            }
            write!(f, ">")?;
        }
        Ok(())
    }
}

impl BcsEncode for StructTag {
    fn bcs_encode(&self, writer: &mut BcsWriter) {
        self.address.bcs_encode(writer);
        self.module.bcs_encode(writer);
        self.name.bcs_encode(writer);
        self.type_args.bcs_encode(writer);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bcs_encoding::to_bytes;

    /// Guards against accidental reordering of the enum: each variant index
    /// is part of the wire format.
    #[test]
    fn test_type_tag_variant_indices_are_pinned() {
        let cases: Vec<(TypeTag, u8)> = vec![
            (TypeTag::Bool, 0),
            (TypeTag::U8, 1),
            (TypeTag::U64, 2),
            (TypeTag::U128, 3),
            (TypeTag::Address, 4),
            (TypeTag::Signer, 5),
            (TypeTag::Vector(Box::new(TypeTag::U8)), 6),
            (TypeTag::Struct(Box::new(StructTag::aptos_coin())), 7),
            (TypeTag::U16, 8),
            (TypeTag::U32, 9),
            (TypeTag::U256, 10),
        ];
        for (tag, index) in cases {
            assert_eq!(to_bytes(&tag)[0], index, "index of {tag:?}");
        }
    }

    #[test]
    fn test_vector_type_tag_is_recursive() {
        let tag = TypeTag::Vector(Box::new(TypeTag::Vector(Box::new(TypeTag::U64))));
        assert_eq!(to_bytes(&tag), vec![6, 6, 2]);
        assert_eq!(tag.to_string(), "vector<vector<u64>>");
    }

    #[test]
    fn test_aptos_coin_struct_tag_encoding() {
        // 07 || 32B(0x...01) || 0a "aptos_coin" || 09 "AptosCoin" || 00
        let tag = TypeTag::Struct(Box::new(StructTag::aptos_coin()));
        assert_eq!(
            hex::encode(to_bytes(&tag)),
            "0700000000000000000000000000000000000000000000000000000000000000010a6170746f735f636f696e094170746f73436f696e00"
        );
        assert_eq!(tag.to_string(), "0x0000000000000000000000000000000000000000000000000000000000000001::aptos_coin::AptosCoin");
    }

    #[test]
    fn test_generic_struct_tag_display() {
        let tag = StructTag::new(
            AccountAddress::ONE,
            Identifier::new("coin").unwrap(),
            Identifier::new("CoinStore").unwrap(),
            vec![TypeTag::Struct(Box::new(StructTag::aptos_coin()))],
        );
        assert!(tag.to_string().starts_with("0x00"));
        assert!(tag.to_string().contains("::coin::CoinStore<"));
    }
}
