//! Minimal hand-rolled BCS (Binary Canonical Serialization) writer shared by
//! the Aptos and Sui modules.
//!
//! BCS rules implemented here (see <https://github.com/diem/bcs>):
//! - Fixed-width integers are little-endian, no framing.
//! - Sequence lengths, byte/string lengths and enum variant indices are
//!   canonical ULEB128.
//! - `bool` is a single byte `0x00`/`0x01`; `Option` is a `0x00`/`0x01` tag
//!   followed by the value.
//! - Structs are their fields concatenated in declaration order; fixed-size
//!   byte arrays are raw bytes without a length prefix.

/// An append-only BCS output buffer.
#[derive(Debug, Default)]
pub struct BcsWriter {
    buf: Vec<u8>,
}

impl BcsWriter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Consume the writer and return the encoded bytes.
    pub fn into_bytes(self) -> Vec<u8> {
        self.buf
    }

    pub fn write_u8(&mut self, value: u8) {
        self.buf.push(value);
    }

    pub fn write_u16(&mut self, value: u16) {
        self.buf.extend_from_slice(&value.to_le_bytes());
    }

    pub fn write_u32(&mut self, value: u32) {
        self.buf.extend_from_slice(&value.to_le_bytes());
    }

    pub fn write_u64(&mut self, value: u64) {
        self.buf.extend_from_slice(&value.to_le_bytes());
    }

    pub fn write_u128(&mut self, value: u128) {
        self.buf.extend_from_slice(&value.to_le_bytes());
    }

    pub fn write_bool(&mut self, value: bool) {
        self.buf.push(u8::from(value));
    }

    /// Write a canonical ULEB128-encoded unsigned integer.
    pub fn write_uleb128(&mut self, mut value: u32) {
        loop {
            let mut byte = (value & 0x7f) as u8;
            value >>= 7;
            if value != 0 {
                byte |= 0x80;
            }
            self.buf.push(byte);
            if value == 0 {
                break;
            }
        }
    }

    /// Write a sequence/bytes/string length prefix.
    ///
    /// # Panics
    ///
    /// Panics if `len` exceeds `u32::MAX`, the maximum BCS sequence length.
    pub fn write_len(&mut self, len: usize) {
        let len = u32::try_from(len).expect("BCS sequence length exceeds u32::MAX");
        self.write_uleb128(len);
    }

    /// Write an enum variant index.
    pub fn write_variant(&mut self, index: u32) {
        self.write_uleb128(index);
    }

    /// Write raw bytes without a length prefix (fixed-size arrays, digests).
    pub fn write_fixed(&mut self, bytes: &[u8]) {
        self.buf.extend_from_slice(bytes);
    }

    /// Write length-prefixed bytes (`Vec<u8>`).
    pub fn write_bytes(&mut self, bytes: &[u8]) {
        self.write_len(bytes.len());
        self.write_fixed(bytes);
    }

    /// Write a length-prefixed UTF-8 string.
    pub fn write_string(&mut self, value: &str) {
        self.write_bytes(value.as_bytes());
    }

    /// Write an `Option` tag; the caller writes the value when `true`.
    pub fn write_option_tag(&mut self, is_some: bool) {
        self.buf.push(u8::from(is_some));
    }
}

/// Types that know how to append their canonical BCS encoding to a writer.
pub trait BcsEncode {
    fn bcs_encode(&self, writer: &mut BcsWriter);
}

impl<T: BcsEncode> BcsEncode for [T] {
    fn bcs_encode(&self, writer: &mut BcsWriter) {
        writer.write_len(self.len());
        for item in self {
            item.bcs_encode(writer);
        }
    }
}

impl<T: BcsEncode> BcsEncode for Vec<T> {
    fn bcs_encode(&self, writer: &mut BcsWriter) {
        self.as_slice().bcs_encode(writer);
    }
}

impl<T: BcsEncode> BcsEncode for Option<T> {
    fn bcs_encode(&self, writer: &mut BcsWriter) {
        match self {
            None => writer.write_option_tag(false),
            Some(value) => {
                writer.write_option_tag(true);
                value.bcs_encode(writer);
            }
        }
    }
}

/// Encode a value to a standalone BCS byte vector.
pub fn to_bytes<T: BcsEncode>(value: &T) -> Vec<u8> {
    let mut writer = BcsWriter::new();
    value.bcs_encode(&mut writer);
    writer.into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn uleb128(value: u32) -> Vec<u8> {
        let mut w = BcsWriter::new();
        w.write_uleb128(value);
        w.into_bytes()
    }

    #[test]
    fn test_uleb128_boundaries() {
        assert_eq!(uleb128(0), vec![0x00]);
        assert_eq!(uleb128(1), vec![0x01]);
        assert_eq!(uleb128(127), vec![0x7f]);
        assert_eq!(uleb128(128), vec![0x80, 0x01]);
        assert_eq!(uleb128(16383), vec![0xff, 0x7f]);
        assert_eq!(uleb128(16384), vec![0x80, 0x80, 0x01]);
        assert_eq!(uleb128(u32::MAX), vec![0xff, 0xff, 0xff, 0xff, 0x0f]);
    }

    #[test]
    fn test_primitives_match_reference_bcs() {
        let mut w = BcsWriter::new();
        w.write_u8(0xab);
        assert_eq!(w.into_bytes(), bcs::to_bytes(&0xab_u8).unwrap());

        let mut w = BcsWriter::new();
        w.write_u16(0xabcd);
        assert_eq!(w.into_bytes(), bcs::to_bytes(&0xabcd_u16).unwrap());

        let mut w = BcsWriter::new();
        w.write_u32(0xdead_beef);
        assert_eq!(w.into_bytes(), bcs::to_bytes(&0xdead_beef_u32).unwrap());

        let mut w = BcsWriter::new();
        w.write_u64(0x0123_4567_89ab_cdef);
        assert_eq!(
            w.into_bytes(),
            bcs::to_bytes(&0x0123_4567_89ab_cdef_u64).unwrap()
        );

        let mut w = BcsWriter::new();
        w.write_u128(0x0123_4567_89ab_cdef_0123_4567_89ab_cdef);
        assert_eq!(
            w.into_bytes(),
            bcs::to_bytes(&0x0123_4567_89ab_cdef_0123_4567_89ab_cdef_u128).unwrap()
        );

        let mut w = BcsWriter::new();
        w.write_bool(true);
        w.write_bool(false);
        assert_eq!(w.into_bytes(), vec![0x01, 0x00]);
    }

    #[test]
    fn test_bytes_string_and_option_match_reference_bcs() {
        let payload: Vec<u8> = (0..200).map(|i| i as u8).collect();
        let mut w = BcsWriter::new();
        w.write_bytes(&payload);
        assert_eq!(w.into_bytes(), bcs::to_bytes(&payload).unwrap());

        let text = "omni-transaction";
        let mut w = BcsWriter::new();
        w.write_string(text);
        assert_eq!(w.into_bytes(), bcs::to_bytes(&text).unwrap());

        let some: Option<u64> = Some(42);
        let mut w = BcsWriter::new();
        w.write_option_tag(true);
        w.write_u64(42);
        assert_eq!(w.into_bytes(), bcs::to_bytes(&some).unwrap());

        let none: Option<u64> = None;
        let mut w = BcsWriter::new();
        w.write_option_tag(false);
        assert_eq!(w.into_bytes(), bcs::to_bytes(&none).unwrap());
    }

    #[test]
    fn test_long_sequence_length_prefix_matches_reference_bcs() {
        let long: Vec<u8> = vec![7u8; 16384];
        let mut w = BcsWriter::new();
        w.write_bytes(&long);
        assert_eq!(w.into_bytes(), bcs::to_bytes(&long).unwrap());
    }
}
