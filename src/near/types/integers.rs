use borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::serde::{Deserialize, Deserializer, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize, Serialize)]
pub struct U64(pub u64);

#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize, Serialize)]
pub struct U128(pub u128);

impl From<u64> for U64 {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl From<u128> for U128 {
    fn from(value: u128) -> Self {
        Self(value)
    }
}

impl<'de> Deserialize<'de> for U64 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct StringOrNumberVisitor;

        impl<'de> serde::de::Visitor<'de> for StringOrNumberVisitor {
            type Value = U64;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string or a number")
            }

            fn visit_str<E>(self, value: &str) -> Result<U64, E>
            where
                E: serde::de::Error,
            {
                value
                    .parse::<u64>()
                    .map(U64)
                    .map_err(serde::de::Error::custom)
            }

            fn visit_u64<E>(self, value: u64) -> Result<U64, E>
            where
                E: serde::de::Error,
            {
                Ok(U64(value))
            }
        }

        deserializer.deserialize_any(StringOrNumberVisitor)
    }
}

impl<'de> Deserialize<'de> for U128 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct StringOrNumberVisitor;

        impl<'de> serde::de::Visitor<'de> for StringOrNumberVisitor {
            type Value = U128;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string or a number 128")
            }

            fn visit_str<E>(self, value: &str) -> Result<U128, E>
            where
                E: serde::de::Error,
            {
                value
                    .parse::<u128>()
                    .map(U128)
                    .map_err(serde::de::Error::custom)
            }

            fn visit_u64<E>(self, value: u64) -> Result<U128, E>
            where
                E: serde::de::Error,
            {
                Ok(U128(value as u128))
            }

            fn visit_u128<E>(self, value: u128) -> Result<U128, E>
            where
                E: serde::de::Error,
            {
                Ok(U128(value))
            }
        }

        deserializer.deserialize_any(StringOrNumberVisitor)
    }
}
