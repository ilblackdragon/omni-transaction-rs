//! Aptos transaction payloads (`TransactionPayload`, `EntryFunction`,
//! `Script`, `Multisig`).
use super::address::AccountAddress;
use super::identifier::{Identifier, ModuleId};
use super::type_tag::TypeTag;
use crate::bcs_encoding::{BcsEncode, BcsWriter};
#[cfg(feature = "schemars")]
use schemars::JsonSchema;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// The payload of an Aptos transaction: what the transaction executes.
///
/// BCS encodes the ULEB128 variant index followed by the variant payload.
/// The indices are frozen by aptos-core's `TransactionPayload`
/// (`types/src/transaction/mod.rs`):
///
/// | index | variant                                          |
/// |-------|--------------------------------------------------|
/// | 0     | `Script`                                         |
/// | 1     | `ModuleBundle` (deprecated, rejected by nodes — intentionally not implemented, index reserved) |
/// | 2     | `EntryFunction`                                  |
/// | 3     | `Multisig`                                       |
///
/// Indices 4+ exist upstream (orderless-transaction work) and are out of
/// scope; never reuse their indices.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub enum TransactionPayload {
    /// A Move script: compiled bytecode executed as the transaction
    /// (BCS variant index 0).
    Script(Script),
    /// An entry function call such as `0x1::aptos_account::transfer`
    /// (BCS variant index 2 — index 1 is the reserved, deprecated
    /// `ModuleBundle`).
    EntryFunction(EntryFunction),
    /// A transaction executed on behalf of an on-chain multisig account
    /// (BCS variant index 3).
    Multisig(Multisig),
}

impl BcsEncode for TransactionPayload {
    fn bcs_encode(&self, writer: &mut BcsWriter) {
        match self {
            Self::Script(script) => {
                writer.write_variant(0);
                script.bcs_encode(writer);
            }
            // Index 1 (ModuleBundle) is deprecated and intentionally skipped.
            Self::EntryFunction(entry_function) => {
                writer.write_variant(2);
                entry_function.bcs_encode(writer);
            }
            Self::Multisig(multisig) => {
                writer.write_variant(3);
                multisig.bcs_encode(writer);
            }
        }
    }
}

/// An entry function call (aptos-core `types/src/transaction/script.rs`).
///
/// In BCS: module id, function name, ULEB128-counted type arguments, then the
/// arguments as a ULEB128-counted sequence of length-prefixed byte strings.
///
/// Each element of `args` is itself the **BCS encoding of the argument
/// value** — e.g. a `u64` amount is 8 little-endian bytes, an address is 32
/// raw bytes; the ULEB128 length prefix of each element is added by this
/// encoder.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct EntryFunction {
    /// The module declaring the function.
    pub module: ModuleId,
    /// The function name.
    pub function: Identifier,
    /// Generic type arguments.
    pub ty_args: Vec<TypeTag>,
    /// BCS-encoded argument values.
    pub args: Vec<Vec<u8>>,
}

impl EntryFunction {
    /// Creates an entry function payload.
    pub const fn new(
        module: ModuleId,
        function: Identifier,
        ty_args: Vec<TypeTag>,
        args: Vec<Vec<u8>>,
    ) -> Self {
        Self {
            module,
            function,
            ty_args,
            args,
        }
    }
}

impl BcsEncode for EntryFunction {
    fn bcs_encode(&self, writer: &mut BcsWriter) {
        self.module.bcs_encode(writer);
        self.function.bcs_encode(writer);
        self.ty_args.bcs_encode(writer);
        writer.write_len(self.args.len());
        for arg in &self.args {
            writer.write_bytes(arg);
        }
    }
}

/// A Move script payload (aptos-core `types/src/transaction/script.rs`).
///
/// In BCS: length-prefixed compiled bytecode, ULEB128-counted type
/// arguments, then ULEB128-counted [`TransactionArgument`]s.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct Script {
    /// Compiled Move bytecode.
    pub code: Vec<u8>,
    /// Generic type arguments.
    pub ty_args: Vec<TypeTag>,
    /// Typed script arguments.
    pub args: Vec<TransactionArgument>,
}

impl Script {
    /// Creates a script payload.
    pub const fn new(code: Vec<u8>, ty_args: Vec<TypeTag>, args: Vec<TransactionArgument>) -> Self {
        Self {
            code,
            ty_args,
            args,
        }
    }
}

impl BcsEncode for Script {
    fn bcs_encode(&self, writer: &mut BcsWriter) {
        writer.write_bytes(&self.code);
        self.ty_args.bcs_encode(writer);
        self.args.bcs_encode(writer);
    }
}

/// A typed argument for a [`Script`] payload.
///
/// BCS encodes the ULEB128 variant index followed by the value. The indices
/// are frozen by aptos-core's `TransactionArgument`
/// (`types/src/transaction/mod.rs`):
///
/// | index | variant    | encoding                        |
/// |-------|------------|---------------------------------|
/// | 0     | `U8`       | 1 byte                          |
/// | 1     | `U64`      | 8 bytes little-endian           |
/// | 2     | `U128`     | 16 bytes little-endian          |
/// | 3     | `Address`  | 32 raw bytes                    |
/// | 4     | `U8Vector` | ULEB128 length + raw bytes      |
/// | 5     | `Bool`     | `0x00`/`0x01`                   |
/// | 6     | `U16`      | 2 bytes little-endian           |
/// | 7     | `U32`      | 4 bytes little-endian           |
/// | 8     | `U256`     | 32 bytes little-endian          |
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub enum TransactionArgument {
    /// A `u8` value (index 0).
    U8(u8),
    /// A `u64` value (index 1).
    U64(u64),
    /// A `u128` value (index 2).
    U128(u128),
    /// An account address (index 3).
    Address(AccountAddress),
    /// A byte vector (index 4).
    U8Vector(Vec<u8>),
    /// A `bool` value (index 5).
    Bool(bool),
    /// A `u16` value (index 6).
    U16(u16),
    /// A `u32` value (index 7).
    U32(u32),
    /// A `u256` value as 32 **little-endian** bytes (index 8).
    U256([u8; 32]),
}

impl BcsEncode for TransactionArgument {
    fn bcs_encode(&self, writer: &mut BcsWriter) {
        match self {
            Self::U8(value) => {
                writer.write_variant(0);
                writer.write_u8(*value);
            }
            Self::U64(value) => {
                writer.write_variant(1);
                writer.write_u64(*value);
            }
            Self::U128(value) => {
                writer.write_variant(2);
                writer.write_u128(*value);
            }
            Self::Address(address) => {
                writer.write_variant(3);
                address.bcs_encode(writer);
            }
            Self::U8Vector(bytes) => {
                writer.write_variant(4);
                writer.write_bytes(bytes);
            }
            Self::Bool(value) => {
                writer.write_variant(5);
                writer.write_bool(*value);
            }
            Self::U16(value) => {
                writer.write_variant(6);
                writer.write_u16(*value);
            }
            Self::U32(value) => {
                writer.write_variant(7);
                writer.write_u32(*value);
            }
            Self::U256(le_bytes) => {
                writer.write_variant(8);
                writer.write_fixed(le_bytes);
            }
        }
    }
}

/// A multisig-account transaction payload
/// (aptos-core `types/src/transaction/multisig.rs`).
///
/// In BCS: 32 raw address bytes, then an `Option` (`0x00`/`0x01` tag) of
/// [`MultisigTransactionPayload`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct Multisig {
    /// The on-chain multisig account executing the transaction.
    pub multisig_address: AccountAddress,
    /// The payload to execute, or `None` to execute the payload already
    /// staged on-chain.
    pub transaction_payload: Option<MultisigTransactionPayload>,
}

impl Multisig {
    /// Creates a multisig payload.
    pub const fn new(
        multisig_address: AccountAddress,
        transaction_payload: Option<MultisigTransactionPayload>,
    ) -> Self {
        Self {
            multisig_address,
            transaction_payload,
        }
    }
}

impl BcsEncode for Multisig {
    fn bcs_encode(&self, writer: &mut BcsWriter) {
        self.multisig_address.bcs_encode(writer);
        self.transaction_payload.bcs_encode(writer);
    }
}

/// The inner payload of a [`Multisig`] transaction.
///
/// BCS variant indices: `EntryFunction` = 0, `Script` = 1.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub enum MultisigTransactionPayload {
    /// An entry function call (index 0).
    EntryFunction(EntryFunction),
    /// A Move script (index 1).
    Script(Script),
}

impl BcsEncode for MultisigTransactionPayload {
    fn bcs_encode(&self, writer: &mut BcsWriter) {
        match self {
            Self::EntryFunction(entry_function) => {
                writer.write_variant(0);
                entry_function.bcs_encode(writer);
            }
            Self::Script(script) => {
                writer.write_variant(1);
                script.bcs_encode(writer);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bcs_encoding::to_bytes;

    fn transfer_entry_function() -> EntryFunction {
        // 0x1222::aptos_coin::transfer(addr 0xdd, u64 1) — the payload of the
        // official TS SDK signing-message vector.
        let receiver = AccountAddress::from_hex("0xdd").unwrap();
        EntryFunction::new(
            ModuleId::new(
                AccountAddress::from_hex("0x1222").unwrap(),
                Identifier::new("aptos_coin").unwrap(),
            ),
            Identifier::new("transfer").unwrap(),
            vec![],
            vec![to_bytes(&receiver), 1u64.to_le_bytes().to_vec()],
        )
    }

    /// Guards against accidental reordering: payload variant indices are the
    /// wire format (index 1, ModuleBundle, is reserved and skipped).
    #[test]
    fn test_transaction_payload_variant_indices_are_pinned() {
        let script = TransactionPayload::Script(Script::new(vec![], vec![], vec![]));
        assert_eq!(to_bytes(&script)[0], 0);

        let entry = TransactionPayload::EntryFunction(transfer_entry_function());
        assert_eq!(to_bytes(&entry)[0], 2);

        let multisig = TransactionPayload::Multisig(Multisig::new(AccountAddress::ONE, None));
        assert_eq!(to_bytes(&multisig)[0], 3);
    }

    #[test]
    fn test_transaction_argument_variant_indices_and_encodings_are_pinned() {
        let cases: Vec<(TransactionArgument, &str)> = vec![
            (TransactionArgument::U8(2), "0002"),
            (TransactionArgument::U64(1), "010100000000000000"),
            (
                TransactionArgument::U128(1),
                "0201000000000000000000000000000000",
            ),
            (
                TransactionArgument::Address(AccountAddress::ONE),
                "030000000000000000000000000000000000000000000000000000000000000001",
            ),
            (TransactionArgument::U8Vector(vec![0xca, 0xfe]), "0402cafe"),
            (TransactionArgument::Bool(true), "0501"),
            // The u16/u32/u256 encodings below are byte-for-byte the args of
            // the official TS SDK "new integer types" script vector.
            (TransactionArgument::U16(0xf111), "0611f1"),
            (TransactionArgument::U32(0xf111_1111), "07111111f1"),
            (
                {
                    let mut le = [0x11u8; 32];
                    le[31] = 0xf1;
                    TransactionArgument::U256(le)
                },
                "0811111111111111111111111111111111111111111111111111111111111111f1",
            ),
        ];
        for (arg, expected_hex) in cases {
            assert_eq!(hex::encode(to_bytes(&arg)), expected_hex, "{arg:?}");
        }
    }

    #[test]
    fn test_entry_function_encoding_matches_official_vector_fragment() {
        // Payload fragment of the official signing-message vector:
        // 02 || module || function || ty_args || args
        let payload = TransactionPayload::EntryFunction(transfer_entry_function());
        assert_eq!(
            hex::encode(to_bytes(&payload)),
            "0200000000000000000000000000000000000000000000000000000000000012220a6170746f735f636f696e087472616e7366657200022000000000000000000000000000000000000000000000000000000000000000dd080100000000000000"
        );
    }

    #[test]
    fn test_entry_function_args_match_reference_bcs_vec_of_bytes() {
        let entry_function = transfer_entry_function();
        let encoded = to_bytes(&entry_function);
        // The trailing args bytes must equal the reference bcs crate's
        // encoding of Vec<Vec<u8>>.
        let reference_args = bcs::to_bytes(&entry_function.args).unwrap();
        assert!(encoded.ends_with(&reference_args));
    }

    #[test]
    fn test_multisig_option_encoding() {
        let none = Multisig::new(AccountAddress::ONE, None);
        let mut expected = AccountAddress::ONE.into_inner().to_vec();
        expected.push(0x00);
        assert_eq!(to_bytes(&none), expected);

        let some = Multisig::new(
            AccountAddress::ONE,
            Some(MultisigTransactionPayload::EntryFunction(
                transfer_entry_function(),
            )),
        );
        let encoded = to_bytes(&some);
        assert_eq!(encoded[32], 0x01); // Option::Some tag
        assert_eq!(encoded[33], 0x00); // MultisigTransactionPayload::EntryFunction index
    }
}
