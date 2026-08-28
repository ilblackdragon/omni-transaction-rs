//! Object references and gas data for Sui transactions.
use core::fmt;

use super::address::{ObjectID, SuiAddress};
use crate::bcs_encoding::{BcsEncode, BcsWriter};
#[cfg(feature = "schemars")]
use schemars::JsonSchema;
#[cfg(feature = "serde")]
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Byte length of an [`ObjectDigest`].
pub const OBJECT_DIGEST_LENGTH: usize = 32;

/// The 32-byte digest of a Sui object, exchanged as base58 by RPC nodes and
/// explorers.
///
/// # BCS
///
/// An object digest is **not** a fixed array on the wire: it serializes as
/// BCS `bytes`, i.e. a ULEB128 length prefix (`0x20`) followed by the 32
/// digest bytes — 33 bytes total. Encoding it as a bare 32-byte array shifts
/// every subsequent byte of the transaction.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ObjectDigest(pub [u8; OBJECT_DIGEST_LENGTH]);

impl ObjectDigest {
    /// Creates a digest from raw bytes.
    pub const fn new(bytes: [u8; OBJECT_DIGEST_LENGTH]) -> Self {
        Self(bytes)
    }

    /// Returns the raw 32-byte representation.
    pub const fn into_inner(self) -> [u8; OBJECT_DIGEST_LENGTH] {
        self.0
    }

    /// Parses a digest from its base58 string form.
    ///
    /// # Errors
    ///
    /// Returns [`DigestParseError`] if the input is not valid base58 or does
    /// not decode to exactly 32 bytes.
    pub fn from_base58(base58_str: &str) -> Result<Self, DigestParseError> {
        let bytes = bs58::decode(base58_str)
            .into_vec()
            .map_err(|_| DigestParseError)?;
        let bytes: [u8; OBJECT_DIGEST_LENGTH] = bytes.try_into().map_err(|_| DigestParseError)?;
        Ok(Self(bytes))
    }

    /// Renders the digest as a base58 string.
    pub fn to_base58(&self) -> String {
        bs58::encode(&self.0).into_string()
    }
}

impl fmt::Display for ObjectDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_base58())
    }
}

impl From<[u8; OBJECT_DIGEST_LENGTH]> for ObjectDigest {
    fn from(bytes: [u8; OBJECT_DIGEST_LENGTH]) -> Self {
        Self(bytes)
    }
}

impl core::str::FromStr for ObjectDigest {
    type Err = DigestParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_base58(s)
    }
}

impl BcsEncode for ObjectDigest {
    fn bcs_encode(&self, writer: &mut BcsWriter) {
        // Serialized as BCS `bytes` (ULEB length 0x20 + 32 bytes), NOT as a
        // fixed array. See the type-level docs.
        writer.write_bytes(&self.0);
    }
}

/// Error returned when parsing an [`ObjectDigest`] from base58 fails.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DigestParseError;

impl fmt::Display for DigestParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "invalid Sui object digest: expected base58 of exactly 32 bytes"
        )
    }
}

impl std::error::Error for DigestParseError {}

// The JSON form of a digest is a base58 string, matching how RPC nodes and
// explorers exchange digests, so serde is implemented by hand.
#[cfg(feature = "serde")]
impl Serialize for ObjectDigest {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_base58())
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for ObjectDigest {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct DigestVisitor;

        impl<'de> serde::de::Visitor<'de> for DigestVisitor {
            type Value = ObjectDigest;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a base58 string or an array of 32 bytes")
            }

            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
                ObjectDigest::from_base58(v).map_err(E::custom)
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let mut bytes = [0u8; OBJECT_DIGEST_LENGTH];
                for (i, byte) in bytes.iter_mut().enumerate() {
                    *byte = seq
                        .next_element::<u8>()?
                        .ok_or_else(|| serde::de::Error::invalid_length(i, &"32 bytes"))?;
                }
                if seq.next_element::<u8>()?.is_some() {
                    return Err(serde::de::Error::custom("expected exactly 32 bytes"));
                }
                Ok(ObjectDigest(bytes))
            }
        }

        deserializer.deserialize_any(DigestVisitor)
    }
}

// Schema mirrors the serde form (base58 string).
#[cfg(feature = "schemars")]
impl JsonSchema for ObjectDigest {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "ObjectDigest".into()
    }

    fn json_schema(generator: &mut schemars::generate::SchemaGenerator) -> schemars::Schema {
        <String>::json_schema(generator)
    }
}

/// A reference to a specific version of an on-chain object:
/// `(object_id, version, digest)`.
///
/// # BCS
///
/// The three fields are concatenated: 32 raw id bytes, `u64` little-endian
/// version, then the length-prefixed digest (see [`ObjectDigest`]).
///
/// Object references must be fetched from RPC at build time and become stale
/// if the object is mutated before the transaction lands (equivocation risk
/// when the same gas coin is signed into two transactions).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct ObjectRef {
    /// The 32-byte object identifier.
    pub object_id: ObjectID,
    /// The object version (sequence number).
    pub version: u64,
    /// The digest of that object version.
    pub digest: ObjectDigest,
}

impl ObjectRef {
    /// Creates a new object reference.
    pub const fn new(object_id: ObjectID, version: u64, digest: ObjectDigest) -> Self {
        Self {
            object_id,
            version,
            digest,
        }
    }
}

impl BcsEncode for ObjectRef {
    fn bcs_encode(&self, writer: &mut BcsWriter) {
        self.object_id.bcs_encode(writer);
        writer.write_u64(self.version);
        self.digest.bcs_encode(writer);
    }
}

/// Gas payment information of a Sui transaction.
///
/// # BCS
///
/// `vector<ObjectRef> payment || 32-byte owner || u64 price || u64 budget`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct GasData {
    /// The coin objects paying for gas (all must be `Coin<SUI>`).
    pub payment: Vec<ObjectRef>,
    /// Owner of the gas objects: the sender or a sponsor.
    pub owner: SuiAddress,
    /// Gas price in MIST per gas unit (at least the reference gas price).
    pub price: u64,
    /// Maximum gas budget in MIST for this transaction.
    pub budget: u64,
}

impl BcsEncode for GasData {
    fn bcs_encode(&self, writer: &mut BcsWriter) {
        self.payment.bcs_encode(writer);
        self.owner.bcs_encode(writer);
        writer.write_u64(self.price);
        writer.write_u64(self.budget);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bcs_encoding::to_bytes;

    /// Spec probe vector: digests are ULEB-prefixed 33-byte `bytes`, not a
    /// fixed 32-byte array.
    #[test]
    fn test_object_digest_bcs_is_length_prefixed() {
        let digest =
            ObjectDigest::from_base58("7gyGAp71YXQRoxmFBaHxofQXAipvgHyBKPyxmdSJxyvz").unwrap();
        assert_eq!(digest, ObjectDigest::new([0x63u8; 32]));

        let encoded = to_bytes(&digest);
        assert_eq!(encoded.len(), 33);
        assert_eq!(
            hex::encode(&encoded),
            "206363636363636363636363636363636363636363636363636363636363636363"
        );

        // Cross-check against the reference sui-sdk-types encoding.
        let reference = sui_sdk_types::Digest::new([0x63u8; 32]);
        assert_eq!(encoded, ::bcs::to_bytes(&reference).unwrap());
    }

    #[test]
    fn test_object_digest_base58_round_trip() {
        let digest = ObjectDigest::new([0x42u8; 32]);
        let base58 = digest.to_base58();
        assert_eq!(base58, "5TeWSsjg2gbxCyWVniXeCmwM7UtHTCK7svzJr5xYJzHf");
        assert_eq!(ObjectDigest::from_base58(&base58).unwrap(), digest);
        assert!(ObjectDigest::from_base58("abc").is_err());
        assert!(ObjectDigest::from_base58("l0O").is_err());
    }

    #[test]
    fn test_object_ref_matches_reference_encoding() {
        let object_ref = ObjectRef::new(
            SuiAddress::new([0x11u8; 32]),
            5,
            ObjectDigest::new([0x63u8; 32]),
        );
        let reference = sui_sdk_types::ObjectReference::new(
            sui_sdk_types::Address::new([0x11u8; 32]),
            5,
            sui_sdk_types::Digest::new([0x63u8; 32]),
        );
        assert_eq!(to_bytes(&object_ref), ::bcs::to_bytes(&reference).unwrap());
    }

    #[test]
    fn test_gas_data_encoding_layout() {
        let gas_data = GasData {
            payment: vec![ObjectRef::new(
                SuiAddress::from_hex("0x1").unwrap(),
                2,
                ObjectDigest::new([0x63u8; 32]),
            )],
            owner: SuiAddress::from_hex("0x2").unwrap(),
            price: 1000,
            budget: 5_000_000,
        };
        let encoded = to_bytes(&gas_data);
        // 1 (vec len) + 32 (id) + 8 (version) + 33 (digest) + 32 (owner) + 8 + 8
        assert_eq!(encoded.len(), 122);
        assert_eq!(encoded[0], 1);
        assert_eq!(
            &encoded[74..106],
            SuiAddress::from_hex("0x2").unwrap().as_bytes()
        );
        assert_eq!(&encoded[106..114], &1000u64.to_le_bytes());
        assert_eq!(&encoded[114..122], &5_000_000u64.to_le_bytes());
    }
}
