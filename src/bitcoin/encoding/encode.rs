use std::io::{BufRead, Write};

use super::{
    decode::Decodable,
    extensions::{ReadExt, WriteExt},
    macros::{impl_array, impl_int_encodable},
    utils::encode_with_size,
};

/// Data which can be encoded in a bitcoin-consistent way.
pub trait Encodable {
    /// Encodes an object with a well-defined format.
    ///
    /// # Returns
    ///
    /// The number of bytes written on success. The only errors returned are errors propagated from
    /// the writer.
    fn encode<W: Write + ?Sized>(&self, writer: &mut W) -> Result<usize, std::io::Error>;
}

// Encodable implementations for arrays
impl_array!(32);

// Encodable implementations for integer types
impl_int_encodable!(u8, read_u8, emit_u8);
impl_int_encodable!(u16, read_u16, emit_u16);
impl_int_encodable!(u32, read_u32, emit_u32);
impl_int_encodable!(u64, read_u64, emit_u64);

// Encodable implementation for `Vec<u8>`
impl Encodable for Vec<u8> {
    fn encode<W: Write + ?Sized>(&self, w: &mut W) -> Result<usize, std::io::Error> {
        encode_with_size(self, w)
    }
}
