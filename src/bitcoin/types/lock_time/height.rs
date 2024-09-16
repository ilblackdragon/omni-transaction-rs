use super::constants::LOCK_TIME_THRESHOLD;

/// An absolute block height, guaranteed to always contain a valid height value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Height(u32);

impl Height {
    /// Absolute block height 0, the genesis block.
    pub const ZERO: Self = Height(0);

    /// The minimum absolute block height (0), the genesis block.
    pub const MIN: Self = Self::ZERO;

    /// The maximum absolute block height.
    pub const MAX: Self = Height(LOCK_TIME_THRESHOLD - 1);

    /// Constructs a new block height.
    ///
    /// # Errors
    ///
    /// If `n` does not represent a valid block height value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use omni_transaction::locktime::Height;
    ///
    /// let h: u32 = 741521;
    /// let height = Height::from_u32(h).expect("invalid height value");
    /// assert_eq!(height.to_u32(), h);
    /// ```
    pub fn from_u32(n: u32) -> Result<Height, String> {
        if is_block_height(n) {
            Ok(Self(n))
        } else {
            Err(format!("Invalid height value: {}", n))
        }
    }

    /// Converts this [`Height`] to its inner `u32` value.
    pub fn to_u32(self) -> u32 {
        self.0
    }
}

/// Returns true if `n` is a block height i.e., less than 500,000,000.
pub fn is_block_height(n: u32) -> bool {
    n < LOCK_TIME_THRESHOLD
}

impl<'de> serde::Deserialize<'de> for Height {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let u = <u32 as serde::Deserialize>::deserialize(deserializer)?;
        Height::from_u32(u).map_err(serde::de::Error::custom)
    }
}

impl serde::Serialize for Height {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_u32(self.to_u32())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zero_height() {
        assert_eq!(Height::ZERO, Height(0));
    }

    #[test]
    fn test_min_height() {
        assert_eq!(Height::MIN, Height::ZERO);
    }

    #[test]
    fn test_max_height() {
        assert_eq!(Height::MAX, Height(LOCK_TIME_THRESHOLD - 1));
    }

    #[test]
    fn test_from_u32() {
        assert_eq!(Height::from_u32(0), Ok(Height::ZERO));
        assert_eq!(Height::from_u32(1), Ok(Height(1)));
        assert_eq!(Height::from_u32(LOCK_TIME_THRESHOLD - 1), Ok(Height::MAX));
        assert_eq!(
            Height::from_u32(LOCK_TIME_THRESHOLD),
            Err("Invalid height value: 500000000".to_string())
        );
    }

    #[test]
    fn test_from_u32_invalid() {
        assert_eq!(
            Height::from_u32(LOCK_TIME_THRESHOLD),
            Err("Invalid height value: 500000000".to_string())
        );
    }

    #[test]
    fn test_to_u32() {
        assert_eq!(Height::ZERO.to_u32(), 0);
        assert_eq!(Height(1).to_u32(), 1);
        assert_eq!(Height::MAX.to_u32(), LOCK_TIME_THRESHOLD - 1);
    }

    #[test]
    fn test_to_u32_invalid() {
        assert_eq!(Height::MAX.to_u32(), LOCK_TIME_THRESHOLD - 1);
    }

    #[test]
    fn test_serder_serialization_roundtrip() {
        let height = Height::from_u32(1).unwrap();
        let serialized = serde_json::to_string(&height).unwrap();
        assert_eq!(serialized, "1");

        let deserialized: Height = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, height);
    }
}
