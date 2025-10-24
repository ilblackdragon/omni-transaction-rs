//! no_std compatible I/O traits for Bitcoin encoding
//!
//! This module provides traits and types that replace std::io for Bitcoin
//! serialization/deserialization in no_std environments.

use alloc::vec::Vec;
use core::fmt;

/// Error type for encoding/decoding operations
#[derive(Debug, Clone)]
pub enum Error {
    /// Unexpected end of input
    UnexpectedEof,
    /// Invalid data encountered
    InvalidData(&'static str),
    /// Non-minimal VarInt encoding
    NonMinimalVarInt,
    /// Oversized vector allocation
    OversizedVectorAllocation,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::UnexpectedEof => write!(f, "unexpected end of file"),
            Error::InvalidData(msg) => write!(f, "invalid data: {}", msg),
            Error::NonMinimalVarInt => write!(f, "non-minimal varint"),
            Error::OversizedVectorAllocation => write!(f, "oversized vector allocation"),
        }
    }
}

/// Trait for writing bytes (no_std replacement for std::io::Write)
pub trait Write {
    /// Write all bytes from buf into this writer
    fn write_all(&mut self, buf: &[u8]) -> Result<(), Error>;
}

/// Trait for reading bytes (no_std replacement for std::io::Read)
pub trait Read {
    /// Read exact number of bytes to fill buf
    fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), Error>;
}

/// Trait for buffered reading (no_std replacement for std::io::BufRead)
pub trait BufRead: Read {}

/// Implements Write for Vec<u8>
impl Write for Vec<u8> {
    fn write_all(&mut self, buf: &[u8]) -> Result<(), Error> {
        self.extend_from_slice(buf);
        Ok(())
    }
}

/// Implements Write for mutable byte slice
impl Write for &mut [u8] {
    fn write_all(&mut self, buf: &[u8]) -> Result<(), Error> {
        if self.len() < buf.len() {
            return Err(Error::InvalidData("buffer too small"));
        }
        let (dest, rest) = core::mem::take(self).split_at_mut(buf.len());
        dest.copy_from_slice(buf);
        *self = rest;
        Ok(())
    }
}

/// Implement Read for &[u8] directly
impl Read for &[u8] {
    fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), Error> {
        if self.len() < buf.len() {
            return Err(Error::UnexpectedEof);
        }
        let (data, rest) = self.split_at(buf.len());
        buf.copy_from_slice(data);
        *self = rest;
        Ok(())
    }
}

impl BufRead for &[u8] {}
