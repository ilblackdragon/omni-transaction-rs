use std::io::Write;

use super::{extensions::WriteExt, macros::impl_to_u64, Encodable};

/// A conversion trait for unsigned integer types smaller than or equal to 64-bits.
///
/// This trait exists because [`usize`] doesn't implement `Into<u64>`.
pub trait ToU64 {
    /// Converts unsigned integer type to a [`u64`].
    fn to_u64(self) -> u64;
}

impl_to_u64!(u8, u16, u32, u64);

macro_rules! const_assert {
    ($x:expr $(; $message:expr)?) => {
        const _: () = {
            if !$x {
                // We can't use formatting in const, only concating literals.
                panic!(concat!("assertion ", stringify!($x), " failed" $(, ": ", $message)?))
            }
        };
    }
}

impl ToU64 for usize {
    fn to_u64(self) -> u64 {
        const_assert!(
            core::mem::size_of::<usize>() <= 8;
            "platforms that have usize larger than 64 bits are not supported"
        );
        self as u64
    }
}

/// A variable-length integer type.
pub struct VarInt(u64);

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

// Global utility functions
pub fn encode_with_size<W: Write + ?Sized>(
    data: &[u8],
    w: &mut W,
) -> Result<usize, std::io::Error> {
    let vi_len = VarInt(data.len().to_u64()).encode(w)?;
    w.emit_slice(data)?;
    Ok(vi_len + data.len())
}
