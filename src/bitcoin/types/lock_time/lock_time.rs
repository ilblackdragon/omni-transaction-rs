use super::{height::Height, time::Time};
use borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::serde::{Deserialize, Serialize};

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
