//! Coin amounts in nanotons.
use core::fmt;

/// An amount of Toncoin in nanotons (1 TON = 10^9 nanotons).
///
/// On the wire it is a `Grams` / `VarUInteger 16`: a 4-bit byte length
/// followed by that many big-endian value bytes, so the maximum
/// representable amount is `2^120 - 1` (15 bytes), not `u128::MAX`
/// (see [`crate::ton::types::CellBuilder::store_coins`]).
///
/// In JSON it is a **decimal string** (like this crate's NEAR integer
/// types): nanoton amounts routinely exceed `2^53`, which JavaScript
/// clients silently corrupt when they arrive as JSON numbers. Numbers are
/// still accepted on input.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Coins(pub u128);

impl Coins {
    /// Creates an amount from nanotons.
    pub const fn from_nano(nanotons: u128) -> Self {
        Self(nanotons)
    }

    /// Returns the amount in nanotons.
    pub const fn as_nano(self) -> u128 {
        self.0
    }

    /// Returns the minimal number of bytes needed to store the value
    /// (0 for a zero amount, per the minimal-length `VarUInteger` rule).
    pub(crate) const fn byte_len(self) -> u8 {
        ((128 - self.0.leading_zeros()).div_ceil(8)) as u8
    }
}

impl From<u128> for Coins {
    fn from(nanotons: u128) -> Self {
        Self(nanotons)
    }
}

impl fmt::Display for Coins {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// Serialization emits a decimal string so that amounts above 2^53 survive
// JSON tooling (JavaScript clients would round a bare number); the wire
// format is the hand-rolled `Grams` encoding and never derived from serde.
#[cfg(feature = "serde")]
impl serde::Serialize for Coins {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.0.to_string())
    }
}

// Deserialization accepts both JSON numbers and decimal strings (u128
// amounts routinely exceed the safe-integer range of JSON tooling).
#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Coins {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error as DeError;

        struct CoinsVisitor;

        impl serde::de::Visitor<'_> for CoinsVisitor {
            type Value = Coins;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a nanoton amount as a number or decimal string")
            }

            fn visit_u64<E: DeError>(self, v: u64) -> Result<Self::Value, E> {
                Ok(Coins(v.into()))
            }

            fn visit_u128<E: DeError>(self, v: u128) -> Result<Self::Value, E> {
                Ok(Coins(v))
            }

            fn visit_str<E: DeError>(self, v: &str) -> Result<Self::Value, E> {
                v.parse::<u128>()
                    .map(Coins)
                    .map_err(|_| DeError::custom(format!("invalid nanoton amount: {v}")))
            }
        }

        deserializer.deserialize_any(CoinsVisitor)
    }
}

// The schema mirrors the serde form (decimal string), which a structural
// derive would not, hence the manual impl.
#[cfg(feature = "schemars")]
impl schemars::JsonSchema for Coins {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "Coins".into()
    }

    fn json_schema(generator: &mut schemars::generate::SchemaGenerator) -> schemars::Schema {
        <String>::json_schema(generator)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_byte_len_is_minimal() {
        assert_eq!(Coins(0).byte_len(), 0);
        assert_eq!(Coins(1).byte_len(), 1);
        assert_eq!(Coins(255).byte_len(), 1);
        assert_eq!(Coins(256).byte_len(), 2);
        assert_eq!(Coins(50_000_000).byte_len(), 4);
        assert_eq!(Coins((1 << 120) - 1).byte_len(), 15);
        assert_eq!(Coins(u128::MAX).byte_len(), 16);
    }

    #[test]
    fn test_conversions() {
        assert_eq!(Coins::from_nano(7).as_nano(), 7);
        assert_eq!(Coins::from(9u128), Coins(9));
        assert_eq!(Coins(42).to_string(), "42");
    }

    #[cfg(feature = "serde_json")]
    #[test]
    fn test_serde_accepts_numbers_and_strings() {
        let coins: Coins = serde_json::from_str("50000000").unwrap();
        assert_eq!(coins, Coins(50_000_000));
        let coins: Coins =
            serde_json::from_str("\"340282366920938463463374607431768211455\"").unwrap();
        assert_eq!(coins, Coins(u128::MAX));
    }

    /// Amounts are serialized as decimal strings, so values above 2^53
    /// survive a round trip through JSON tooling untouched.
    #[cfg(feature = "serde_json")]
    #[test]
    fn test_serde_emits_decimal_strings() {
        assert_eq!(serde_json::to_string(&Coins(5)).unwrap(), "\"5\"");
        for amount in [
            0,
            1,
            50_000_000,
            (1u128 << 53) + 1,
            (1u128 << 120) - 1,
            u128::MAX,
        ] {
            let json = serde_json::to_string(&Coins(amount)).unwrap();
            assert_eq!(json, format!("\"{amount}\""));
            let back: Coins = serde_json::from_str(&json).unwrap();
            assert_eq!(back, Coins(amount));
            // A JSON number is still accepted on the way in, as far as JSON
            // integers reach (serde_json parses larger literals as floats,
            // which is exactly the corruption the string form avoids).
            if amount <= u128::from(u64::MAX) {
                let from_number: Coins = serde_json::from_str(&amount.to_string()).unwrap();
                assert_eq!(from_number, Coins(amount));
            } else {
                assert!(serde_json::from_str::<Coins>(&amount.to_string()).is_err());
            }
        }
    }
}
