use super::constants::LOCK_TIME_THRESHOLD;

/// A UNIX timestamp, seconds since epoch, guaranteed to always contain a valid time value.
///
/// Note that `Time(x)` means 'x seconds since epoch' _not_ '(x - threshold) seconds since epoch'.
#[derive(Debug, PartialEq, Eq)]
pub struct Time(u32);

impl Time {
    /// The minimum absolute block time (Tue Nov 05 1985 00:53:20 GMT+0000).
    pub const MIN: u32 = LOCK_TIME_THRESHOLD;

    /// The maximum absolute block time (Sun Feb 07 2106 06:28:15 GMT+0000).
    pub const MAX: u32 = u32::MAX;

    /// Returns true if `n` is a valid UNIX timestamp i.e., greater than or equal to 500,000,000.
    pub fn is_valid(n: u32) -> bool {
        n >= LOCK_TIME_THRESHOLD
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_min_time() {
        assert_eq!(Time::MIN, LOCK_TIME_THRESHOLD);
    }

    #[test]
    fn test_max_time() {
        assert_eq!(Time::MAX, u32::MAX);
    }

    #[test]
    fn test_is_valid() {
        assert!(Time::is_valid(LOCK_TIME_THRESHOLD));
        assert!(!Time::is_valid(LOCK_TIME_THRESHOLD - 1));
    }

    #[test]
    fn test_is_valid_when_passed_invalid_value() {
        assert!(!Time::is_valid(LOCK_TIME_THRESHOLD - 1));
    }
}
