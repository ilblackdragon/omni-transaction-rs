use core::fmt;
use std::str::FromStr;

#[cfg(feature = "serde")]
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

use crate::constants::ED25519_PUBLIC_KEY_LENGTH;
use crate::solana::utils::decode_base58_fixed;

/// A Solana account address: a raw 32-byte ed25519 public key.
///
/// Conventionally displayed (and serialized to JSON) as a base58 string,
/// e.g. `AKnL4NNf3DGWZJS6cPknBuEGnVsV4A4m5tgebLHaRSZ9`.
///
/// The derived `Ord` implementation compares the raw 32 bytes
/// lexicographically, which is exactly the order the Solana SDK uses when
/// compiling account keys into a message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct SolanaAddress(pub [u8; ED25519_PUBLIC_KEY_LENGTH]);

impl SolanaAddress {
    /// Parses an address from its base58 string representation.
    pub fn from_base58(s: &str) -> Result<Self, String> {
        decode_base58_fixed::<ED25519_PUBLIC_KEY_LENGTH>(s).map(Self)
    }

    /// Returns the base58 string representation of the address.
    pub fn to_base58(&self) -> String {
        bs58::encode(&self.0).into_string()
    }

    /// Returns the raw 32 bytes of the address.
    pub const fn to_bytes(&self) -> [u8; ED25519_PUBLIC_KEY_LENGTH] {
        self.0
    }
}

impl From<[u8; ED25519_PUBLIC_KEY_LENGTH]> for SolanaAddress {
    fn from(bytes: [u8; ED25519_PUBLIC_KEY_LENGTH]) -> Self {
        Self(bytes)
    }
}

impl FromStr for SolanaAddress {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_base58(s)
    }
}

impl fmt::Display for SolanaAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_base58())
    }
}

// Serialized as a base58 string (the canonical Solana JSON representation);
// deserialization also accepts an array of 32 bytes for flexibility, in line
// with the NEAR `BlockHash` type of this crate.
#[cfg(feature = "serde")]
impl Serialize for SolanaAddress {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_base58())
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for SolanaAddress {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer
            .deserialize_any(Base58Visitor::<32>("base58 string or 32-byte array"))
            .map(Self)
    }
}

// Hand-implemented as a string schema to match the base58 serde
// representation above (the derive would describe a 32-element array), same
// pattern as the NEAR `Secp256K1Signature` type of this crate.
#[cfg(feature = "schemars")]
impl schemars::JsonSchema for SolanaAddress {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "SolanaAddress".into()
    }

    fn json_schema(generator: &mut schemars::generate::SchemaGenerator) -> schemars::Schema {
        <String>::json_schema(generator)
    }
}

/// A recent blockhash: the raw 32 bytes of a Solana block hash.
///
/// Conventionally displayed (and serialized to JSON) as a base58 string,
/// e.g. `EETubP5AKHgjPAhzPAFcb8BAY1hMH639CWCFTqi3hq1k`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Blockhash(pub [u8; 32]);

impl Blockhash {
    /// Parses a blockhash from its base58 string representation.
    pub fn from_base58(s: &str) -> Result<Self, String> {
        decode_base58_fixed::<32>(s).map(Self)
    }

    /// Returns the base58 string representation of the blockhash.
    pub fn to_base58(&self) -> String {
        bs58::encode(&self.0).into_string()
    }

    /// Returns the raw 32 bytes of the blockhash.
    pub const fn to_bytes(&self) -> [u8; 32] {
        self.0
    }
}

impl From<[u8; 32]> for Blockhash {
    fn from(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
}

impl FromStr for Blockhash {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_base58(s)
    }
}

impl fmt::Display for Blockhash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_base58())
    }
}

#[cfg(feature = "serde")]
impl Serialize for Blockhash {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_base58())
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for Blockhash {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer
            .deserialize_any(Base58Visitor::<32>("base58 string or 32-byte array"))
            .map(Self)
    }
}

// Hand-implemented as a string schema for the same reason as
// `SolanaAddress` above.
#[cfg(feature = "schemars")]
impl schemars::JsonSchema for Blockhash {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "Blockhash".into()
    }

    fn json_schema(generator: &mut schemars::generate::SchemaGenerator) -> schemars::Schema {
        <String>::json_schema(generator)
    }
}

/// Serde visitor accepting either a base58 string or a sequence of `N` bytes.
///
/// Not re-exported: the `address` module is private, so this stays
/// crate-internal despite the `pub` visibility.
#[cfg(feature = "serde")]
pub struct Base58Visitor<const N: usize>(pub &'static str);

#[cfg(feature = "serde")]
impl<'de, const N: usize> de::Visitor<'de> for Base58Visitor<N> {
    type Value = [u8; N];

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str(self.0)
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        decode_base58_fixed::<N>(value).map_err(de::Error::custom)
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: de::SeqAccess<'de>,
    {
        let mut arr = [0u8; N];
        for (i, elem) in arr.iter_mut().enumerate() {
            *elem = seq
                .next_element::<u8>()?
                .ok_or_else(|| de::Error::invalid_length(i, &self))?;
        }
        // A sequence longer than `N` must be rejected here rather than left
        // to the data format: keeping only the first `N` bytes would yield a
        // *different* key than the one supplied, and a format whose
        // `SeqAccess` does not verify that the visitor drained it would
        // accept that silently. Same idiom as the Aptos/Sui address types.
        if seq.next_element::<u8>()?.is_some() {
            return Err(de::Error::custom(format!("expected exactly {N} bytes")));
        }
        Ok(arr)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_address_base58_round_trip() {
        let address =
            SolanaAddress::from_base58("AKnL4NNf3DGWZJS6cPknBuEGnVsV4A4m5tgebLHaRSZ9").unwrap();
        assert_eq!(
            hex::encode(address.to_bytes()),
            "8a88e3dd7409f195fd52db2d3cba5d72ca6709bf1d94121bf3748801b40f6f5c"
        );
        assert_eq!(
            address.to_base58(),
            "AKnL4NNf3DGWZJS6cPknBuEGnVsV4A4m5tgebLHaRSZ9"
        );
        assert_eq!(
            address.to_string(),
            "AKnL4NNf3DGWZJS6cPknBuEGnVsV4A4m5tgebLHaRSZ9"
        );
    }

    #[test]
    fn test_address_parsing_against_solana_pubkey() {
        use std::str::FromStr;
        for s in [
            "4vJ9JU1bJJE96FWSJKvHsmmFADCg4gpZQff4P3bkLKi",
            "8opHzTAnfzRpPEx21XtnrVTX28YQuCpAjcn1PczScKh",
            "MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr",
            "SysvarRent111111111111111111111111111111111",
            "11111111111111111111111111111111",
        ] {
            let ours = SolanaAddress::from_base58(s).unwrap();
            let reference = solana_pubkey::Pubkey::from_str(s).unwrap();
            assert_eq!(ours.to_bytes(), reference.to_bytes(), "mismatch for {s}");
            assert_eq!(ours.to_base58(), reference.to_string());
        }
    }

    #[test]
    fn test_address_from_invalid_base58() {
        assert!(SolanaAddress::from_base58("not-base58-0OIl").is_err());
        // Valid base58 but wrong length.
        assert!(SolanaAddress::from_base58("abc").is_err());
    }

    #[test]
    fn test_blockhash_base58_round_trip() {
        let blockhash =
            Blockhash::from_base58("EETubP5AKHgjPAhzPAFcb8BAY1hMH639CWCFTqi3hq1k").unwrap();
        assert_eq!(
            hex::encode(blockhash.to_bytes()),
            "c49ae77603782054f17a9decea43b444eba0edb12c6f1d31c6e0e4a84bf052eb"
        );
        assert_eq!(
            blockhash.to_base58(),
            "EETubP5AKHgjPAhzPAFcb8BAY1hMH639CWCFTqi3hq1k"
        );
        let all_zeros = Blockhash::from_base58("11111111111111111111111111111111").unwrap();
        assert_eq!(all_zeros.to_bytes(), [0u8; 32]);
    }

    #[test]
    #[cfg(feature = "serde_json")]
    fn test_address_serde_round_trip() {
        let address =
            SolanaAddress::from_base58("AKnL4NNf3DGWZJS6cPknBuEGnVsV4A4m5tgebLHaRSZ9").unwrap();
        let json = serde_json::to_string(&address).unwrap();
        assert_eq!(json, "\"AKnL4NNf3DGWZJS6cPknBuEGnVsV4A4m5tgebLHaRSZ9\"");
        let parsed: SolanaAddress = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, address);

        // Byte-array form is accepted too.
        let bytes_json = serde_json::to_string(&address.to_bytes().to_vec()).unwrap();
        let parsed_from_bytes: SolanaAddress = serde_json::from_str(&bytes_json).unwrap();
        assert_eq!(parsed_from_bytes, address);
    }

    /// A byte-array form of the wrong length must be rejected by the visitor
    /// itself, with an error naming the expected length (`serde_json` would
    /// otherwise report an over-long array as "trailing characters", and a
    /// format that does not check for undrained sequence elements would
    /// silently truncate to a *different* key).
    #[test]
    #[cfg(feature = "serde_json")]
    fn test_address_serde_rejects_wrong_length_byte_array() {
        let exact = (0u16..32).map(|i| i.to_string()).collect::<Vec<_>>();
        let exact_json = format!("[{}]", exact.join(","));
        let parsed: SolanaAddress = serde_json::from_str(&exact_json).unwrap();
        assert_eq!(parsed.to_bytes()[31], 31);

        // One element too many: must not yield the first 32 bytes.
        let too_long = (0u16..33).map(|i| i.to_string()).collect::<Vec<_>>();
        let err = serde_json::from_str::<SolanaAddress>(&format!("[{}]", too_long.join(",")))
            .unwrap_err();
        assert!(
            err.to_string().contains("expected exactly 32 bytes"),
            "unexpected error: {err}"
        );

        // Well past the end (the case reported: 40 elements).
        let way_too_long = (0u16..40).map(|i| i.to_string()).collect::<Vec<_>>();
        assert!(
            serde_json::from_str::<SolanaAddress>(&format!("[{}]", way_too_long.join(",")))
                .is_err()
        );

        // One element too few.
        let too_short = (0u16..31).map(|i| i.to_string()).collect::<Vec<_>>();
        assert!(
            serde_json::from_str::<SolanaAddress>(&format!("[{}]", too_short.join(","))).is_err()
        );
    }

    #[test]
    #[cfg(feature = "serde_json")]
    fn test_blockhash_serde_rejects_wrong_length_byte_array() {
        let exact = (0u16..32).map(|i| i.to_string()).collect::<Vec<_>>();
        assert!(serde_json::from_str::<Blockhash>(&format!("[{}]", exact.join(","))).is_ok());
        let too_long = (0u16..33).map(|i| i.to_string()).collect::<Vec<_>>();
        assert!(serde_json::from_str::<Blockhash>(&format!("[{}]", too_long.join(","))).is_err());
        let too_short = (0u16..31).map(|i| i.to_string()).collect::<Vec<_>>();
        assert!(serde_json::from_str::<Blockhash>(&format!("[{}]", too_short.join(","))).is_err());
    }

    #[test]
    #[cfg(feature = "serde_json")]
    fn test_blockhash_serde_round_trip() {
        let blockhash =
            Blockhash::from_base58("EETubP5AKHgjPAhzPAFcb8BAY1hMH639CWCFTqi3hq1k").unwrap();
        let json = serde_json::to_string(&blockhash).unwrap();
        assert_eq!(json, "\"EETubP5AKHgjPAhzPAFcb8BAY1hMH639CWCFTqi3hq1k\"");
        let parsed: Blockhash = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, blockhash);
    }
}
