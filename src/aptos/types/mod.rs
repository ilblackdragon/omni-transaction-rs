//! Minimal required Aptos types, mirroring `aptos-core`'s `types` and
//! `move-core-types` wire formats.
mod address;
mod authenticator;
mod identifier;
mod payload;
mod type_tag;

pub use self::address::AccountAddress;
pub use self::address::AddressParseError;
pub use self::address::ACCOUNT_ADDRESS_LENGTH;
pub use self::authenticator::Ed25519PublicKey;
pub use self::authenticator::Ed25519Signature;
pub use self::authenticator::KeyBytesParseError;
pub use self::authenticator::TransactionAuthenticator;
pub use self::authenticator::ED25519_PUBLIC_KEY_LENGTH;
pub use self::authenticator::ED25519_SIGNATURE_LENGTH;
pub use self::identifier::Identifier;
pub use self::identifier::IdentifierParseError;
pub use self::identifier::ModuleId;
pub use self::identifier::MAX_IDENTIFIER_LENGTH;
pub use self::payload::EntryFunction;
pub use self::payload::Multisig;
pub use self::payload::MultisigTransactionPayload;
pub use self::payload::Script;
pub use self::payload::TransactionArgument;
pub use self::payload::TransactionPayload;
pub use self::type_tag::StructTag;
pub use self::type_tag::TypeTag;
