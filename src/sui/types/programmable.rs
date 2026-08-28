//! Programmable transaction block (PTB) types: inputs, commands and
//! arguments.
use super::address::{ObjectID, SuiAddress};
use super::object::ObjectRef;
use super::type_tag::{Identifier, TypeTag};
use crate::bcs_encoding::{BcsEncode, BcsWriter};
#[cfg(feature = "schemars")]
use schemars::JsonSchema;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// The kind of transaction being built.
///
/// Only user-buildable kinds are modelled; Sui defines further system-only
/// variants (`ChangeEpoch`, `Genesis`, ...) at indices `0x01..` that this
/// library never produces.
///
/// # BCS
///
/// `%x00` followed by the [`ProgrammableTransaction`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub enum TransactionKind {
    /// A user transaction: a list of inputs and commands executed in order.
    ProgrammableTransaction(ProgrammableTransaction),
}

impl BcsEncode for TransactionKind {
    fn bcs_encode(&self, writer: &mut BcsWriter) {
        match self {
            Self::ProgrammableTransaction(pt) => {
                writer.write_variant(0);
                pt.bcs_encode(writer);
            }
        }
    }
}

/// A programmable transaction block: shared `inputs` referenced by index from
/// a list of `commands` executed sequentially.
///
/// # BCS
///
/// `vector<CallArg> inputs || vector<Command> commands`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct ProgrammableTransaction {
    /// Input objects or pure (primitive) values.
    pub inputs: Vec<CallArg>,
    /// Commands executed in order; any failure aborts the whole transaction.
    pub commands: Vec<Command>,
}

impl BcsEncode for ProgrammableTransaction {
    fn bcs_encode(&self, writer: &mut BcsWriter) {
        self.inputs.bcs_encode(writer);
        self.commands.bcs_encode(writer);
    }
}

/// An input to a programmable transaction.
///
/// # BCS
///
/// `%x00 bytes` for `Pure`, `%x01` + [`ObjectArg`] for `Object`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub enum CallArg {
    /// A Move value already serialized as BCS.
    ///
    /// The wrapped bytes must **themselves** be the BCS encoding of the Move
    /// value (a `u64` is 8 little-endian bytes, an `address` is 32 raw
    /// bytes, ...). Prefer the typed [`CallArg::pure_u64`],
    /// [`CallArg::pure_address`], ... constructors, which encode correctly.
    Pure(Vec<u8>),
    /// An on-chain object.
    Object(ObjectArg),
}

impl CallArg {
    /// Builds a `Pure` input from a closure writing the BCS of a Move value.
    fn pure_with(encode: impl FnOnce(&mut BcsWriter)) -> Self {
        let mut writer = BcsWriter::new();
        encode(&mut writer);
        Self::Pure(writer.into_bytes())
    }

    /// Pure `bool` input (1 byte).
    pub fn pure_bool(value: bool) -> Self {
        Self::pure_with(|w| w.write_bool(value))
    }

    /// Pure `u8` input (1 byte).
    pub fn pure_u8(value: u8) -> Self {
        Self::pure_with(|w| w.write_u8(value))
    }

    /// Pure `u16` input (2 little-endian bytes).
    pub fn pure_u16(value: u16) -> Self {
        Self::pure_with(|w| w.write_u16(value))
    }

    /// Pure `u32` input (4 little-endian bytes).
    pub fn pure_u32(value: u32) -> Self {
        Self::pure_with(|w| w.write_u32(value))
    }

    /// Pure `u64` input (8 little-endian bytes) — coin amounts, epochs, ...
    pub fn pure_u64(value: u64) -> Self {
        Self::pure_with(|w| w.write_u64(value))
    }

    /// Pure `u128` input (16 little-endian bytes).
    pub fn pure_u128(value: u128) -> Self {
        Self::pure_with(|w| w.write_u128(value))
    }

    /// Pure `address` input (32 raw bytes, no length prefix).
    pub fn pure_address(address: SuiAddress) -> Self {
        Self::pure_with(|w| w.write_fixed(address.as_bytes()))
    }

    /// Pure `vector<u8>` input (ULEB128 length prefix + bytes).
    pub fn pure_bytes(bytes: &[u8]) -> Self {
        Self::pure_with(|w| w.write_bytes(bytes))
    }

    /// Pure `string::String` input (ULEB128 length prefix + UTF-8 bytes).
    pub fn pure_string(value: &str) -> Self {
        Self::pure_with(|w| w.write_string(value))
    }
}

impl BcsEncode for CallArg {
    fn bcs_encode(&self, writer: &mut BcsWriter) {
        match self {
            Self::Pure(bytes) => {
                writer.write_variant(0);
                writer.write_bytes(bytes);
            }
            Self::Object(object_arg) => {
                writer.write_variant(1);
                object_arg.bcs_encode(writer);
            }
        }
    }
}

/// An object input to a programmable transaction.
///
/// # BCS
///
/// | variant | encoding |
/// |---------|----------|
/// | `ImmOrOwnedObject` | `%x00` + [`ObjectRef`] |
/// | `SharedObject` | `%x01` + 32-byte id + `u64` version + `bool` |
/// | `Receiving` | `%x02` + [`ObjectRef`] |
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub enum ObjectArg {
    /// An immutable or address-owned object.
    ImmOrOwnedObject(ObjectRef),
    /// A shared object.
    SharedObject {
        /// The 32-byte object identifier.
        id: ObjectID,
        /// The version at which the object became shared.
        initial_shared_version: u64,
        /// Whether the object is taken by mutable reference.
        mutable: bool,
    },
    /// An object received by this transaction.
    Receiving(ObjectRef),
}

impl BcsEncode for ObjectArg {
    fn bcs_encode(&self, writer: &mut BcsWriter) {
        match self {
            Self::ImmOrOwnedObject(object_ref) => {
                writer.write_variant(0);
                object_ref.bcs_encode(writer);
            }
            Self::SharedObject {
                id,
                initial_shared_version,
                mutable,
            } => {
                writer.write_variant(1);
                id.bcs_encode(writer);
                writer.write_u64(*initial_shared_version);
                writer.write_bool(*mutable);
            }
            Self::Receiving(object_ref) => {
                writer.write_variant(2);
                object_ref.bcs_encode(writer);
            }
        }
    }
}

/// A single command of a programmable transaction.
///
/// # BCS
///
/// A ULEB128 variant index (`MoveCall` = `0x00` ... `Upgrade` = `0x06`)
/// followed by the fields of the variant, concatenated in order.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub enum Command {
    /// Call an entry or public Move function. Index `0x00`.
    MoveCall(ProgrammableMoveCall),
    /// Send objects to an address. Index `0x01`.
    TransferObjects {
        /// The objects to transfer.
        objects: Vec<Argument>,
        /// The recipient address (usually a `Pure` address input).
        address: Argument,
    },
    /// Split amounts off a coin, producing new coins. Index `0x02`.
    SplitCoins {
        /// The coin to split ([`Argument::GasCoin`] to split the gas coin).
        coin: Argument,
        /// The amounts to split off (`u64` pure inputs).
        amounts: Vec<Argument>,
    },
    /// Merge coins into the first coin. Index `0x03`.
    MergeCoins {
        /// The coin that receives the merged balances.
        coin: Argument,
        /// The coins merged (and deleted) into `coin`.
        coins_to_merge: Vec<Argument>,
    },
    /// Publish a Move package. Index `0x04`.
    Publish {
        /// The serialized Move modules.
        modules: Vec<Vec<u8>>,
        /// Package ids of the package's transitive dependencies.
        dependencies: Vec<ObjectID>,
    },
    /// Build a Move vector from elements of one type. Index `0x05`.
    MakeMoveVec {
        /// The element type; required when it cannot be inferred (e.g. all
        /// elements are pure inputs, or the vector is empty).
        type_tag: Option<TypeTag>,
        /// The elements of the vector.
        elements: Vec<Argument>,
    },
    /// Upgrade an already published package. Index `0x06`.
    Upgrade {
        /// The serialized Move modules of the new version.
        modules: Vec<Vec<u8>>,
        /// Package ids of the package's transitive dependencies.
        dependencies: Vec<ObjectID>,
        /// The package id being upgraded.
        package: ObjectID,
        /// The `UpgradeTicket` produced by an earlier command.
        ticket: Argument,
    },
}

fn encode_modules(modules: &[Vec<u8>], writer: &mut BcsWriter) {
    writer.write_len(modules.len());
    for module in modules {
        writer.write_bytes(module);
    }
}

impl BcsEncode for Command {
    fn bcs_encode(&self, writer: &mut BcsWriter) {
        match self {
            Self::MoveCall(move_call) => {
                writer.write_variant(0);
                move_call.bcs_encode(writer);
            }
            Self::TransferObjects { objects, address } => {
                writer.write_variant(1);
                objects.bcs_encode(writer);
                address.bcs_encode(writer);
            }
            Self::SplitCoins { coin, amounts } => {
                writer.write_variant(2);
                coin.bcs_encode(writer);
                amounts.bcs_encode(writer);
            }
            Self::MergeCoins {
                coin,
                coins_to_merge,
            } => {
                writer.write_variant(3);
                coin.bcs_encode(writer);
                coins_to_merge.bcs_encode(writer);
            }
            Self::Publish {
                modules,
                dependencies,
            } => {
                writer.write_variant(4);
                encode_modules(modules, writer);
                dependencies.bcs_encode(writer);
            }
            Self::MakeMoveVec { type_tag, elements } => {
                writer.write_variant(5);
                type_tag.bcs_encode(writer);
                elements.bcs_encode(writer);
            }
            Self::Upgrade {
                modules,
                dependencies,
                package,
                ticket,
            } => {
                writer.write_variant(6);
                encode_modules(modules, writer);
                dependencies.bcs_encode(writer);
                package.bcs_encode(writer);
                ticket.bcs_encode(writer);
            }
        }
    }
}

/// A call to an entry or public Move function.
///
/// # BCS
///
/// `32-byte package id || module identifier || function identifier ||
/// vector<TypeTag> || vector<Argument>`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct ProgrammableMoveCall {
    /// The package containing the module.
    pub package: ObjectID,
    /// The module containing the function.
    pub module: Identifier,
    /// The function to call.
    pub function: Identifier,
    /// The type arguments of the function.
    pub type_arguments: Vec<TypeTag>,
    /// The arguments of the function.
    pub arguments: Vec<Argument>,
}

impl BcsEncode for ProgrammableMoveCall {
    fn bcs_encode(&self, writer: &mut BcsWriter) {
        self.package.bcs_encode(writer);
        self.module.bcs_encode(writer);
        self.function.bcs_encode(writer);
        self.type_arguments.bcs_encode(writer);
        self.arguments.bcs_encode(writer);
    }
}

/// An argument to a programmable transaction command.
///
/// # BCS
///
/// A ULEB128 variant index; the `Input`/`Result`/`NestedResult` indices are
/// **`u16` little-endian (2 raw bytes)**, not ULEB128 — `Input(1)` is
/// `0x01 0x01 0x00` and `Input(300)` is `0x01 0x2c 0x01`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub enum Argument {
    /// The gas coin; only usable by-reference except for `TransferObjects`.
    GasCoin,
    /// One of the transaction inputs, by index.
    Input(u16),
    /// The result of an earlier command, by command index.
    Result(u16),
    /// One value of an earlier command returning multiple values:
    /// `(command index, result index)`.
    NestedResult(u16, u16),
}

impl BcsEncode for Argument {
    fn bcs_encode(&self, writer: &mut BcsWriter) {
        match self {
            Self::GasCoin => writer.write_variant(0),
            Self::Input(index) => {
                writer.write_variant(1);
                writer.write_u16(*index);
            }
            Self::Result(index) => {
                writer.write_variant(2);
                writer.write_u16(*index);
            }
            Self::NestedResult(command, result) => {
                writer.write_variant(3);
                writer.write_u16(*command);
                writer.write_u16(*result);
            }
        }
    }
}

/// A time-to-live for a transaction.
///
/// # BCS
///
/// `%x00` for `None`, `%x01` + `u64` little-endian epoch for `Epoch`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub enum TransactionExpiration {
    /// The transaction never expires.
    #[default]
    None,
    /// Validators only sign while the current epoch is at most this value.
    Epoch(u64),
}

impl BcsEncode for TransactionExpiration {
    fn bcs_encode(&self, writer: &mut BcsWriter) {
        match self {
            Self::None => writer.write_variant(0),
            Self::Epoch(epoch) => {
                writer.write_variant(1);
                writer.write_u64(*epoch);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bcs_encoding::to_bytes;

    fn encoded_hex<T: BcsEncode>(value: &T) -> String {
        hex::encode(to_bytes(value))
    }

    #[test]
    fn test_argument_indices_are_u16_little_endian() {
        assert_eq!(encoded_hex(&Argument::GasCoin), "00");
        assert_eq!(encoded_hex(&Argument::Input(0)), "010000");
        assert_eq!(encoded_hex(&Argument::Input(1)), "010100");
        // Indices > 255 catch a ULEB mis-encoding that small indices hide.
        assert_eq!(encoded_hex(&Argument::Input(256)), "010001");
        assert_eq!(encoded_hex(&Argument::Input(300)), "012c01");
        assert_eq!(encoded_hex(&Argument::Result(2)), "020200");
        assert_eq!(encoded_hex(&Argument::NestedResult(1, 300)), "0301002c01");
    }

    #[test]
    fn test_argument_against_reference_sdk() {
        use sui_sdk_types::Argument as RefArgument;
        let cases: [(Argument, RefArgument); 5] = [
            (Argument::GasCoin, RefArgument::Gas),
            (Argument::Input(0), RefArgument::Input(0)),
            (Argument::Input(300), RefArgument::Input(300)),
            (Argument::Result(7), RefArgument::Result(7)),
            (
                Argument::NestedResult(300, 65535),
                RefArgument::NestedResult(300, 65535),
            ),
        ];
        for (ours, reference) in &cases {
            assert_eq!(to_bytes(ours), ::bcs::to_bytes(reference).unwrap());
        }
    }

    #[test]
    fn test_pure_constructors_encode_move_values() {
        assert_eq!(
            CallArg::pure_u64(1_000_000),
            CallArg::Pure(hex::decode("40420f0000000000").unwrap())
        );
        assert_eq!(CallArg::pure_bool(true), CallArg::Pure(vec![0x01]));
        assert_eq!(CallArg::pure_u8(7), CallArg::Pure(vec![0x07]));
        assert_eq!(CallArg::pure_u16(300), CallArg::Pure(vec![0x2c, 0x01]));
        assert_eq!(
            CallArg::pure_u32(1_000_000),
            CallArg::Pure(vec![0x40, 0x42, 0x0f, 0x00])
        );
        assert_eq!(
            CallArg::pure_u128(1),
            CallArg::Pure(::bcs::to_bytes(&1u128).unwrap())
        );
        let address = SuiAddress::from_hex("0x3").unwrap();
        assert_eq!(
            CallArg::pure_address(address),
            CallArg::Pure(address.into_inner().to_vec())
        );
        // vector<u8> and string are ULEB length-prefixed.
        assert_eq!(
            CallArg::pure_bytes(&[1, 2, 3]),
            CallArg::Pure(vec![3, 1, 2, 3])
        );
        assert_eq!(
            CallArg::pure_string("sui"),
            CallArg::Pure(vec![3, b's', b'u', b'i'])
        );
        assert_eq!(
            CallArg::pure_string("sui"),
            CallArg::Pure(::bcs::to_bytes(&"sui").unwrap())
        );
    }

    #[test]
    fn test_expiration_encoding() {
        assert_eq!(encoded_hex(&TransactionExpiration::None), "00");
        assert_eq!(
            encoded_hex(&TransactionExpiration::Epoch(100)),
            "016400000000000000"
        );
        assert_eq!(
            to_bytes(&TransactionExpiration::Epoch(100)),
            ::bcs::to_bytes(&sui_sdk_types::TransactionExpiration::Epoch(100)).unwrap()
        );
        assert_eq!(
            to_bytes(&TransactionExpiration::None),
            ::bcs::to_bytes(&sui_sdk_types::TransactionExpiration::None).unwrap()
        );
    }

    #[test]
    fn test_object_arg_encoding_against_reference_sdk() {
        use super::super::object::{ObjectDigest, ObjectRef};

        let object_ref = ObjectRef::new(
            SuiAddress::new([0x22u8; 32]),
            7,
            ObjectDigest::new([0x42u8; 32]),
        );
        let reference_ref = sui_sdk_types::ObjectReference::new(
            sui_sdk_types::Address::new([0x22u8; 32]),
            7,
            sui_sdk_types::Digest::new([0x42u8; 32]),
        );

        let owned = CallArg::Object(ObjectArg::ImmOrOwnedObject(object_ref));
        let reference = sui_sdk_types::Input::ImmutableOrOwned(reference_ref.clone());
        assert_eq!(to_bytes(&owned), ::bcs::to_bytes(&reference).unwrap());

        let shared = CallArg::Object(ObjectArg::SharedObject {
            id: SuiAddress::from_hex("0x6").unwrap(),
            initial_shared_version: 1,
            mutable: true,
        });
        let reference = sui_sdk_types::Input::Shared(sui_sdk_types::SharedInput::new(
            sui_sdk_types::Address::from_hex("0x6").unwrap(),
            1,
            true,
        ));
        assert_eq!(to_bytes(&shared), ::bcs::to_bytes(&reference).unwrap());

        let receiving = CallArg::Object(ObjectArg::Receiving(object_ref));
        let reference = sui_sdk_types::Input::Receiving(reference_ref);
        assert_eq!(to_bytes(&receiving), ::bcs::to_bytes(&reference).unwrap());
    }
}
