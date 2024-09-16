use super::{height::Height, time::Time};
use borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::serde::{Deserialize, Serialize};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, BorshSerialize, BorshDeserialize,
)]
pub enum LockTime {
    /// A block height lock time value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use omni_transaction::bitcoin::types::LockTime;
    ///
    /// let block: u32 = 741521;
    /// let n = LockTime::from_height(block).expect("valid height");
    /// assert!(n.is_block_height());
    /// assert_eq!(n.to_bytes32(), block);
    /// ```
    Blocks(Height),
    /// A UNIX timestamp lock time value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use omni_transaction::types::LockTime;
    ///
    /// let seconds: u32 = 1653195600; // May 22nd, 5am UTC.
    /// let n = LockTime::from_time(seconds).expect("valid time");
    /// assert!(n.is_block_time());
    /// assert_eq!(n.to_bytes32(), seconds);
    /// ```
    Seconds(Time),
}

impl LockTime {
    pub fn from_height(height: u32) -> Result<Self, String> {
        if height > Height::MAX.to_u32() {
            return Err(format!("Invalid height: {}", height));
        }
        let height = Height::from_u32(height)?;
        Ok(LockTime::Blocks(height))
    }

    pub fn from_time(time: u32) -> Result<Self, String> {
        if time > Time::MAX.to_unix_time() {
            return Err(format!("Invalid time: {}", time));
        }
        let time = Time::from_unix_time(time)?;
        Ok(LockTime::Seconds(time))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bitcoin::types::Height; // Ensure this import is correct

    #[test]
    fn test_from_height() {
        assert_eq!(LockTime::from_height(0), Ok(LockTime::Blocks(Height::ZERO)));
    }
}
