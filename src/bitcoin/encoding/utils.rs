use std::io::{BufRead, Write};

use super::{extensions::WriteExt, macros::impl_to_u64, Decodable, Encodable, ReadExt};

/// A conversion trait for unsigned integer types smaller than or equal to 64-bits.
///
/// This trait exists because [`usize`] doesn't implement `Into<u64>`.
pub trait ToU64 {
    /// Converts unsigned integer type to a [`u64`].
    fn to_u64(self) -> u64;
}

impl_to_u64!(u8, u16, u32, u64);

impl ToU64 for usize {
    fn to_u64(self) -> u64 {
        self as u64
    }
}

/// A variable-length integer type.
pub struct VarInt(pub u64);

impl VarInt {
    /// Returns the number of bytes this varint contributes to a transaction size.
    ///
    /// Returns 1 for 0..=0xFC, 3 for 0xFD..=(2^16-1), 5 for 0x10000..=(2^32-1), and 9 otherwise.
    pub const fn size(&self) -> usize {
        match self.0 {
            0..=0xFC => 1,
            0xFD..=0xFFFF => 3,
            0x10000..=0xFFFFFFFF => 5,
            _ => 9,
        }
    }
}
impl Encodable for VarInt {
    fn encode<W: Write + ?Sized>(&self, w: &mut W) -> Result<usize, std::io::Error> {
        match self.0 {
            0..=0xFC => {
                (self.0 as u8).encode(w)?;
                Ok(1)
            }
            0xFD..=0xFFFF => {
                w.emit_u8(0xFD)?;
                (self.0 as u16).encode(w)?;
                Ok(3)
            }
            0x10000..=0xFFFFFFFF => {
                w.emit_u8(0xFE)?;
                (self.0 as u32).encode(w)?;
                Ok(5)
            }
            _ => {
                w.emit_u8(0xFF)?;
                self.0.encode(w)?;
                Ok(9)
            }
        }
    }
}

/// Implements `From<T> for VarInt`.
///
/// `VarInt`s are consensus encoded as `u64`s so we store them as such. Casting from any integer size smaller than or equal to `u64` is always safe and the cast value is correctly handled by `consensus_encode`.
macro_rules! impl_var_int_from {
    ($($ty:tt),*) => {
        $(
            /// Creates a `VarInt` from a `usize` by casting the to a `u64`.
            impl From<$ty> for VarInt {
                fn from(x: $ty) -> Self { VarInt(x.to_u64()) }
            }
        )*
    }
}
impl_var_int_from!(u8, u16, u32, u64, usize);

impl Decodable for VarInt {
    fn decode<R: BufRead + ?Sized>(r: &mut R) -> Result<Self, std::io::Error> {
        let n = ReadExt::read_u8(r)?;
        match n {
            0xFF => {
                let x = ReadExt::read_u64(r)?;
                if x < 0x100000000 {
                    Err(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "NonMinimalVarInt",
                    ))
                } else {
                    Ok(Self::from(x))
                }
            }
            0xFE => {
                let x = ReadExt::read_u32(r)?;
                if x < 0x10000 {
                    Err(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "NonMinimalVarInt",
                    ))
                } else {
                    Ok(Self::from(x))
                }
            }
            0xFD => {
                let x = ReadExt::read_u16(r)?;
                if x < 0xFD {
                    Err(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "NonMinimalVarInt",
                    ))
                } else {
                    Ok(Self::from(x))
                }
            }
            n => Ok(Self::from(n)),
        }
    }
}

// Global utility functions
pub fn encode_with_size<W: Write + ?Sized>(
    data: &[u8],
    w: &mut W,
) -> Result<usize, std::io::Error> {
    let vi_len = VarInt(data.len().to_u64()).encode(w)?;
    w.emit_slice(data)?;
    Ok(vi_len + data.len())
}
