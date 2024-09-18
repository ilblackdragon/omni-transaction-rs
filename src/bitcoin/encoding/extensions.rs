use std::io::{Read, Write};

use super::macros::{decoder_fn, encoder_fn};

/// Extensions of `Write` to encode data as per Bitcoin specific format.
pub trait WriteExt: Write {
    /// Outputs a 64-bit unsigned integer.
    fn emit_u64(&mut self, v: u64) -> Result<(), std::io::Error>;
    /// Outputs a 32-bit unsigned integer.
    fn emit_u32(&mut self, v: u32) -> Result<(), std::io::Error>;
    /// Outputs a 16-bit unsigned integer.
    fn emit_u16(&mut self, v: u16) -> Result<(), std::io::Error>;
    /// Outputs an 8-bit unsigned integer.
    fn emit_u8(&mut self, v: u8) -> Result<(), std::io::Error>;
    /// Outputs a byte slice.
    fn emit_slice(&mut self, v: &[u8]) -> Result<(), std::io::Error>;
}

/// Extensions of `Read` to decode data as per Bitcoin specific format.
pub trait ReadExt: Read {
    /// Reads a 32-bit unsigned integer.
    fn read_u32(&mut self) -> Result<u32, std::io::Error>;
}

impl<W: Write + ?Sized> WriteExt for W {
    encoder_fn!(emit_u64, u64);
    encoder_fn!(emit_u32, u32);
    encoder_fn!(emit_u16, u16);

    fn emit_u8(&mut self, v: u8) -> Result<(), std::io::Error> {
        self.write_all(&[v])
    }

    fn emit_slice(&mut self, v: &[u8]) -> Result<(), std::io::Error> {
        self.write_all(v)
    }
}

impl<R: Read + ?Sized> ReadExt for R {
    decoder_fn!(read_u32, u32, 4);
}
