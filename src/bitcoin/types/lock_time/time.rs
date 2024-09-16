use super::constants::LOCK_TIME_THRESHOLD;
use borsh::{BorshDeserialize, BorshSerialize};

/// A UNIX timestamp, seconds since epoch, guaranteed to always contain a valid time value.
///
/// Note that `Time(x)` means 'x seconds since epoch' _not_ '(x - threshold) seconds since epoch'.
#[derive(Debug, Clone, Copy, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct Time(u32);

impl Time {
    /// The minimum absolute block time (Tue Nov 05 1985 00:53:20 GMT+0000).
    pub const MIN: Self = Time(LOCK_TIME_THRESHOLD);

    /// The maximum absolute block time (Sun Feb 07 2106 06:28:15 GMT+0000).
    pub const MAX: Self = Time(u32::MAX);

    /// Constructs a new block time.
    ///
    /// # Errors
    ///
    /// If `n` does not encode a valid UNIX time stamp.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use omni_transaction::bitcoin::lock_time::Time;
    ///
    /// let t: u32 = 1653195600; // May 22nd, 5am UTC.
    /// let time = Time::from_unix_time(t).expect("invalid time value");
    /// assert_eq!(time.to_unix_time(), t);
    /// ```
    pub fn from_unix_time(n: u32) -> Result<Time, String> {
        if is_block_time(n) {
            Ok(Self(n))
        } else {
            Err(format!("Invalid time value: {}", n))
        }
    }

    /// Converts this [`Time`] to its inner `u32` value.
    pub fn to_unix_time(self) -> u32 {
        self.0
    }
}

/// Returns true if `n` is a UNIX timestamp i.e., greater than or equal to 500,000,000.
pub fn is_block_time(n: u32) -> bool {
    n >= LOCK_TIME_THRESHOLD
}

impl<'de> serde::Deserialize<'de> for Time {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let u = serde::Deserialize::deserialize(deserializer)?;
        Ok(Time::from_unix_time(u).map_err(serde::de::Error::custom)?)
    }
}

impl serde::Serialize for Time {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_u32(self.to_unix_time())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_min_time() {
        assert_eq!(Time::MIN, Time(LOCK_TIME_THRESHOLD));
    }

    #[test]
    fn test_max_time() {
        assert_eq!(Time::MAX, Time(u32::MAX));
    }

    #[test]
    fn test_from_unix_time() {
        let t: u32 = 1653195600; // May 22nd, 5am UTC.
        let time = Time::from_unix_time(t).expect("invalid time value");

        assert_eq!(time.to_unix_time(), t);
    }

    #[test]
    fn test_from_unix_time_invalid() {
        let t: u32 = 42;
        let time = Time::from_unix_time(t);

        assert_eq!(time, Err(format!("Invalid time value: {}", t)));
    }

    #[test]
    fn test_to_unix_time() {
        let t: u32 = 1653195600; // May 22nd, 5am UTC.
        let time = Time::from_unix_time(t).unwrap();

        assert_eq!(time.to_unix_time(), t);
    }

    #[test]
    fn test_to_unix_time_invalid() {
        let t: u32 = 42;
        let time = Time::from_unix_time(t).unwrap();

        assert_eq!(time.to_unix_time(), t);
    }

    #[test]
    fn test_serde_serialization_roundtrip() {
        let time = Time::from_unix_time(1653195600).unwrap();
        let serialized = serde_json::to_string(&time).unwrap();
        assert_eq!(serialized, "1653195600");

        let deserialized: Time = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, time);
    }
}
