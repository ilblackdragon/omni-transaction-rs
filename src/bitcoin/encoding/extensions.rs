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
    /// Reads a 64-bit unsigned integer.
    fn read_u64(&mut self) -> Result<u64, std::io::Error>;
    /// Reads a 32-bit unsigned integer.
    fn read_u32(&mut self) -> Result<u32, std::io::Error>;
    /// Reads a 16-bit unsigned integer.
    fn read_u16(&mut self) -> Result<u16, std::io::Error>;
    /// Reads an 8-bit unsigned integer.
    fn read_u8(&mut self) -> Result<u8, std::io::Error>;
    /// Reads a byte slice.
    fn read_slice(&mut self, slice: &mut [u8]) -> Result<(), std::io::Error>;
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
    decoder_fn!(read_u64, u64, 8);
    decoder_fn!(read_u32, u32, 4);
    decoder_fn!(read_u16, u16, 2);

    fn read_u8(&mut self) -> Result<u8, std::io::Error> {
        let mut slice = [0u8; 1];
        self.read_exact(&mut slice)?;
        Ok(slice[0])
    }

    fn read_slice(&mut self, slice: &mut [u8]) -> Result<(), std::io::Error> {
        self.read_exact(slice).map_err(std::io::Error::from)
    }
}
