use std::{
    io::{BufRead, Write},
    ops,
};

use borsh::{BorshDeserialize, BorshSerialize};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::bitcoin::encoding::{Decodable, Encodable};

/// An amount.
///
/// The [`Amount`] type can be used to express Bitcoin amounts that support
/// arithmetic and conversion to various denominations.
#[derive(
    Debug,
    Copy,
    Clone,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    BorshSerialize,
    BorshDeserialize,
    JsonSchema,
)]
#[serde(crate = "near_sdk::serde")]
pub struct Amount(u64);

impl Amount {
    /// The zero amount.
    pub const ZERO: Self = Self(0);
    /// Exactly one satoshi.
    pub const ONE_SAT: Self = Self(1);
    /// Exactly one bitcoin.
    pub const ONE_BTC: Self = Self::from_int_btc(1);
    /// The maximum value allowed as an amount. Useful for sanity checking.
    pub const MAX_MONEY: Self = Self::from_int_btc(21_000_000);
    /// The minimum value of an amount.
    pub const MIN: Self = Self::ZERO;
    /// The maximum value of an amount.
    pub const MAX: Self = Self(u64::MAX);
    /// The number of bytes that an amount contributes to the size of a transaction.
    pub const SIZE: usize = 8; // Serialized length of a u64.

    /// Creates an [`Amount`] with satoshi precision and the given number of satoshis.
    pub const fn from_sat(satoshi: u64) -> Self {
        Self(satoshi)
    }

    /// Gets the number of satoshis in this [`Amount`].
    pub const fn to_sat(self) -> u64 {
        self.0
    }

    /// Converts from a value expressing integer values of bitcoins to an [`Amount`]
    /// in const context.
    ///
    /// # Panics
    ///
    /// The function panics if the argument multiplied by the number of sats
    /// per bitcoin overflows a u64 type.
    pub const fn from_int_btc(btc: u64) -> Self {
        match btc.checked_mul(100_000_000) {
            Some(amount) => Self::from_sat(amount),
            None => panic!("checked_mul overflowed"),
        }
    }

    /// Checked addition.
    ///
    /// Returns [`None`] if overflow occurred.
    pub fn checked_add(self, rhs: Self) -> Option<Self> {
        self.0.checked_add(rhs.0).map(Amount)
    }

    /// Checked subtraction.
    ///
    /// Returns [`None`] if overflow occurred.
    pub fn checked_sub(self, rhs: Self) -> Option<Self> {
        self.0.checked_sub(rhs.0).map(Amount)
    }
}

impl Encodable for Amount {
    fn encode<W: Write + ?Sized>(&self, w: &mut W) -> Result<usize, std::io::Error> {
        self.0.encode(w)
    }
}

impl Decodable for Amount {
    fn decode_from_finite_reader<R: BufRead + ?Sized>(r: &mut R) -> Result<Self, std::io::Error> {
        let value = Decodable::decode_from_finite_reader(r)?;
        Ok(Self::from_sat(value))
    }
}

impl ops::Add for Amount {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        self.checked_add(rhs).expect("Amount addition error")
    }
}

impl ops::Sub for Amount {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        self.checked_sub(rhs).expect("Amount subtraction error")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode() {
        let amount = Amount::from_sat(1000);
        let mut buf = Vec::new();
        let size = amount.encode(&mut buf).unwrap();
        assert_eq!(size, Amount::SIZE);

        let decoded_amount = Amount::decode_from_finite_reader(&mut buf.as_slice()).unwrap();
        assert_eq!(decoded_amount, amount);
    }
}
