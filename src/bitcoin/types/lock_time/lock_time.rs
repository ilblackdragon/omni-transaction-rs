use crate::bitcoin::{
    encoding::{Decodable, Encodable},
    types::lock_time::constants::LOCK_TIME_THRESHOLD,
};

use super::{height::Height, time::Time};
use std::{
    fmt,
    io::{BufRead, Write},
};

use borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::serde::{Deserialize, Serialize};
use serde::Deserializer;

/// Locktime itself is an unsigned 4-byte integer which can be parsed two ways:
///
/// If less than 500 million, locktime is parsed as a block height.
/// The transaction can be added to any block which has this height or higher.
///
/// If greater than or equal to 500 million, locktime is parsed using the Unix epoch time format
/// (the number of seconds elapsed since 1970-01-01T00:00 UTC—currently over 1.395 billion).
/// The transaction can be added to any block whose block time is greater than the locktime.
///
/// [Bitcoin Devguide]: https://developer.bitcoin.org/devguide/transactions.html#locktime-and-sequence-number
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, BorshSerialize, BorshDeserialize)]
pub struct LockTime(u32);

impl LockTime {
    /// The number of bytes that the locktime contributes to the size of a transaction.
    pub const SIZE: usize = 4; // Serialized length of a u32.

    pub fn from_height(height: u32) -> Result<Self, String> {
        if Height::is_valid(height) {
            Ok(Self(height))
        } else {
            Err(format!("Invalid block height: {}", height))
        }
    }

    pub fn from_time(time: u32) -> Result<Self, String> {
        if Time::is_valid(time) {
            Ok(Self(time))
        } else {
            Err(format!("Invalid timestamp: {}", time))
        }
    }

    pub const fn is_block_height(&self) -> bool {
        Height::is_valid(self.0)
    }

    pub const fn is_unix_time(&self) -> bool {
        Time::is_valid(self.0)
    }

    pub const fn to_u32(&self) -> u32 {
        self.0
    }
}

impl Encodable for LockTime {
    fn encode<W: Write + ?Sized>(&self, w: &mut W) -> Result<usize, std::io::Error> {
        self.0.encode(w)
    }
}

impl Decodable for LockTime {
    fn decode<R: BufRead + ?Sized>(r: &mut R) -> Result<Self, std::io::Error> {
        // 4 bytes
        let mut buf: [u8; 4] = [0; 4];
        r.read_exact(&mut buf)?;
        Ok(Self(u32::from_le_bytes(buf)))
    }
}

impl<'de> Deserialize<'de> for LockTime {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct StringOrNumberVisitor;

        impl<'de> serde::de::Visitor<'de> for StringOrNumberVisitor {
            type Value = LockTime;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string or a number")
            }

            fn visit_str<E>(self, value: &str) -> Result<LockTime, E>
            where
                E: serde::de::Error,
            {
                if let Ok(value_parsed) = value.parse::<u32>() {
                    if value_parsed < LOCK_TIME_THRESHOLD {
                        return Ok(LockTime::from_height(value_parsed).unwrap());
                    } else {
                        return Ok(LockTime::from_time(value_parsed).unwrap());
                    }
                }
                // if the string is not a valid number, we return an error
                Err(serde::de::Error::custom(
                    "Invalid lock time: expected a number",
                ))
            }

            fn visit_u32<E>(self, value: u32) -> Result<LockTime, E>
            where
                E: serde::de::Error,
            {
                if value < LOCK_TIME_THRESHOLD {
                    Ok(LockTime::from_height(value).unwrap())
                } else {
                    Ok(LockTime::from_time(value).unwrap())
                }
            }

            fn visit_u64<E>(self, value: u64) -> Result<LockTime, E>
            where
                E: serde::de::Error,
            {
                if value < LOCK_TIME_THRESHOLD as u64 {
                    Ok(LockTime::from_height(value as u32).unwrap())
                } else {
                    Ok(LockTime::from_time(value as u32).unwrap())
                }
            }
        }

        deserializer.deserialize_any(StringOrNumberVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bitcoin::types::Height;

    #[test]
    fn test_locktime_size() {
        assert_eq!(LockTime::SIZE, 4);
    }

    #[test]
    fn test_locktime_from_height() {
        let h = 100;
        let height = LockTime::from_height(h).unwrap();

        assert!(height.is_block_height());
        assert!(!height.is_unix_time());
        assert_eq!(height.to_u32(), h);
    }

    #[test]
    fn test_locktime_from_time() {
        let time = LockTime::from_time(Time::MIN + 100).unwrap();

        assert!(!time.is_block_height());
        assert!(time.is_unix_time());
        assert_eq!(time.to_u32(), Time::MIN + 100);
    }

    #[test]
    fn test_locktime_invalid_height() {
        assert!(LockTime::from_height(Height::MAX + 1).is_err());
    }

    #[test]
    fn test_locktime_invalid_time() {
        assert!(LockTime::from_time(Time::MIN - 1).is_err());
    }

    #[test]
    fn test_locktime_serialization() {
        let locktime = LockTime::from_height(100).unwrap();
        let serialized = serde_json::to_string(&locktime).unwrap();
        let deserialized: LockTime = serde_json::from_str(&serialized).unwrap();

        assert_eq!(locktime, deserialized);
    }

    #[test]
    fn test_locktime_borsh_serialization() {
        let locktime = LockTime::from_height(100).unwrap();
        let serialized = borsh::to_vec(&locktime).unwrap();
        let deserialized = LockTime::try_from_slice(&serialized).unwrap();

        assert_eq!(locktime, deserialized);
    }

    #[test]
    fn test_locktime_borsh_serialization_time() {
        let locktime = LockTime::from_time(Time::MIN + 100).unwrap();
        let serialized = borsh::to_vec(&locktime).unwrap();
        let deserialized = LockTime::try_from_slice(&serialized).unwrap();

        assert_eq!(locktime, deserialized);
    }

    #[test]
    fn test_locktime_borsh_serialization_roundtrip() {
        let original = LockTime::from_height(Height::MAX).unwrap();
        let serialized = borsh::to_vec(&original).unwrap();
        let deserialized = LockTime::try_from_slice(&serialized).unwrap();

        assert_eq!(original, deserialized);
        assert_eq!(original.to_u32(), deserialized.to_u32());
    }

    #[test]
    fn test_encode_decode() {
        let locktime = LockTime::from_height(100).unwrap();
        let mut buffer = Vec::new();
        locktime.encode(&mut buffer).unwrap();

        let decoded = LockTime::decode(&mut &buffer[..]).unwrap();
        assert_eq!(locktime, decoded);
    }

    #[test]
    fn test_from_json_locktime() {
        let json = r#"0"#;

        let locktime: LockTime = serde_json::from_str(json).unwrap();
        assert_eq!(locktime, LockTime::from_height(0).unwrap());
    }

    #[test]
    fn test_serde_json_locktime_with_number_as_string() {
        let json = r#""0""#;

        let locktime: LockTime = serde_json::from_str(json).unwrap();
        assert_eq!(locktime, LockTime::from_height(0).unwrap());
    }
}
