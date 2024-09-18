/// Macro to generate encoding functions.
macro_rules! encoder_fn {
    ($name:ident, $val_type:ty) => {
        #[inline]
        fn $name(&mut self, v: $val_type) -> core::result::Result<(), std::io::Error> {
            self.write_all(&v.to_le_bytes())
        }
    };
}

/// Macro to generate decoding functions.
macro_rules! decoder_fn {
    ($name:ident, $val_type:ty, $byte_len:expr) => {
        #[inline]
        fn $name(&mut self) -> core::result::Result<$val_type, std::io::Error> {
            let mut val = [0; $byte_len];
            self.read_exact(&mut val)?;
            Ok(<$val_type>::from_le_bytes(val))
        }
    };
}

/// Macro to implement `Encodable` for fixed-size arrays.
macro_rules! impl_array {
    ($len:expr) => {
        impl Encodable for [u8; $len] {
            fn encode<W: Write + ?Sized>(&self, writer: &mut W) -> Result<usize, std::io::Error> {
                writer.write_all(self)?;
                Ok($len)
            }
        }
    };
}

/// Macro to implement `Encodable` for integer types.
macro_rules! impl_int_encodable {
    ($ty:ty, $read_fn:ident, $emit_fn:ident) => {
        impl Encodable for $ty {
            fn encode<W: Write + ?Sized>(&self, writer: &mut W) -> Result<usize, std::io::Error> {
                writer.$emit_fn(*self)?;
                Ok(std::mem::size_of::<$ty>())
            }
        }
    };
}

/// Macro to implement `ToU64` for various integer types.
macro_rules! impl_to_u64 {
    ($($ty:ident),*) => {
        $(
            impl ToU64 for $ty {
                fn to_u64(self) -> u64 {
                    self.into()
                }
            }
        )*
    }
}

// Export macros for use in other modules
pub(crate) use decoder_fn;
pub(crate) use encoder_fn;
pub(crate) use impl_array;
pub(crate) use impl_int_encodable;
pub(crate) use impl_to_u64;
