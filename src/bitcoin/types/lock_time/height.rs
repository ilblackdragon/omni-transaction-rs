use super::constants::LOCK_TIME_THRESHOLD;

/// An absolute block height, guaranteed to always contain a valid height value.
#[derive(Debug, PartialEq, Eq)]
pub struct Height(u32);

impl Height {
    /// Absolute block height 0, the genesis block.
    pub const ZERO: u32 = 0;

    /// The minimum absolute block height (0), the genesis block.
    pub const MIN: u32 = Height::ZERO;

    /// The maximum absolute block height.
    pub const MAX: u32 = LOCK_TIME_THRESHOLD - 1;

    /// Returns true if `n` is a valid block height i.e., less than 500,000,000.
    pub fn is_valid(n: u32) -> bool {
        n < LOCK_TIME_THRESHOLD
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zero_height() {
        assert_eq!(Height::ZERO, 0);
    }

    #[test]
    fn test_min_height() {
        assert_eq!(Height::MIN, Height::ZERO);
    }

    #[test]
    fn test_max_height() {
        assert_eq!(Height::MAX, LOCK_TIME_THRESHOLD - 1);
    }

    #[test]
    fn test_is_valid() {
        assert!(Height::is_valid(0));
        assert!(Height::is_valid(1));
        assert!(Height::is_valid(LOCK_TIME_THRESHOLD - 1));
        assert!(!Height::is_valid(LOCK_TIME_THRESHOLD));
    }

    #[test]
    fn test_is_valid_when_passed_invalid_value() {
        assert!(!Height::is_valid(LOCK_TIME_THRESHOLD + 1));
    }
}
