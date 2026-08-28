//! TON cell model: ordinary level-0 cells and a bit-level cell builder.
//!
//! A cell holds up to [`MAX_CELL_BITS`] data bits and up to [`MAX_CELL_REFS`]
//! references to other cells. Multi-bit integers are stored big-endian and
//! bits fill bytes MSB-first, per the TVM whitepaper and `block.tlb`.
use core::fmt;
use std::sync::Arc;

use sha2::{Digest, Sha256};

use super::address::TonAddress;
use super::coins::Coins;

/// Maximum number of data bits an ordinary cell can hold.
pub const MAX_CELL_BITS: u16 = 1023;
/// Maximum number of references an ordinary cell can hold.
pub const MAX_CELL_REFS: usize = 4;
/// Maximum depth of a cell tree (TVM requires depth < 1024).
pub const MAX_CELL_DEPTH: u16 = 1023;

/// Error raised while building or decoding a [`Cell`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellError {
    /// The cell data capacity of 1023 bits would be exceeded.
    CellOverflow,
    /// The cell reference capacity of 4 would be exceeded.
    TooManyRefs,
    /// The maximum cell-tree depth of 1023 would be exceeded.
    DepthOverflow,
    /// A value does not fit in the requested bit width (or in a
    /// `VarUInteger 16` for coin amounts).
    ValueOutOfRange,
}

impl fmt::Display for CellError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CellOverflow => write!(f, "cell data capacity of 1023 bits exceeded"),
            Self::TooManyRefs => write!(f, "cell reference capacity of 4 exceeded"),
            Self::DepthOverflow => write!(f, "cell tree depth limit of 1023 exceeded"),
            Self::ValueOutOfRange => write!(f, "value does not fit in the requested bit width"),
        }
    }
}

impl std::error::Error for CellError {}

/// An ordinary level-0 TON cell: up to 1023 data bits plus up to 4
/// references to other cells.
///
/// Cells are immutable; construct them with [`CellBuilder`] or by parsing a
/// Bag of Cells (see [`crate::ton::types::parse_boc`]). The standard
/// representation hash ([`Self::repr_hash`]) and [`Self::depth`] are computed
/// eagerly on construction, so both accessors are O(1).
#[derive(Clone)]
pub struct Cell {
    /// Cell data, padded to a whole number of bytes with the completion tag
    /// (a single `1` bit followed by `0` bits) when `bit_len % 8 != 0`.
    data: Vec<u8>,
    /// Number of meaningful data bits.
    bit_len: u16,
    /// Child cells.
    refs: Vec<Arc<Self>>,
    /// Standard representation hash (TVM whitepaper 3.1.4-3.1.5).
    hash: [u8; 32],
    /// 0 for leaf cells, otherwise `1 + max(depth of children)`.
    depth: u16,
}

impl Cell {
    /// Creates a cell from already-padded parts, computing hash and depth.
    ///
    /// `data` must be exactly `ceil(bit_len / 8)` bytes with the completion
    /// tag already applied when `bit_len % 8 != 0`.
    pub(crate) fn from_parts(
        data: Vec<u8>,
        bit_len: u16,
        refs: Vec<Arc<Self>>,
    ) -> Result<Self, CellError> {
        if bit_len > MAX_CELL_BITS || data.len() != usize::from(bit_len.div_ceil(8)) {
            return Err(CellError::CellOverflow);
        }
        if refs.len() > MAX_CELL_REFS {
            return Err(CellError::TooManyRefs);
        }
        let depth = match refs.iter().map(|r| r.depth).max() {
            None => 0,
            Some(d) if d >= MAX_CELL_DEPTH => return Err(CellError::DepthOverflow),
            Some(d) => d + 1,
        };
        let mut hasher = Sha256::new();
        hasher.update([Self::descriptor_d1(&refs), Self::descriptor_d2(bit_len)]);
        hasher.update(&data);
        for r in &refs {
            hasher.update(r.depth.to_be_bytes());
        }
        for r in &refs {
            hasher.update(r.hash);
        }
        let hash = hasher.finalize().into();
        Ok(Self {
            data,
            bit_len,
            refs,
            hash,
            depth,
        })
    }

    /// Returns the empty cell (0 bits, 0 references).
    pub fn empty() -> Self {
        Self::from_parts(Vec::new(), 0, Vec::new()).expect("empty cell is always valid")
    }

    /// Returns the number of meaningful data bits.
    pub const fn bit_len(&self) -> u16 {
        self.bit_len
    }

    /// Returns the padded cell data (`ceil(bit_len / 8)` bytes; when
    /// `bit_len % 8 != 0` the last byte carries the completion tag).
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Returns the child cells.
    pub fn refs(&self) -> &[Arc<Self>] {
        &self.refs
    }

    /// Returns the standard cell representation hash (SHA-256 over the
    /// refs/data descriptors, padded data, child depths and child hashes).
    ///
    /// This is the value TON smart contracts sign and verify
    /// (`check_signature(slice_hash(..), ..)`).
    pub const fn repr_hash(&self) -> [u8; 32] {
        self.hash
    }

    /// Returns the cell depth: 0 for leaves, `1 + max(child depths)`
    /// otherwise.
    pub const fn depth(&self) -> u16 {
        self.depth
    }

    /// First descriptor byte: reference count (ordinary level-0 cells only).
    pub(crate) const fn descriptor_d1(refs: &[Arc<Self>]) -> u8 {
        refs.len() as u8
    }

    /// Second descriptor byte: `ceil(bits / 8) + floor(bits / 8)`; odd values
    /// signal a completion tag.
    pub(crate) const fn descriptor_d2(bit_len: u16) -> u8 {
        (bit_len.div_ceil(8) + bit_len / 8) as u8
    }
}

impl Default for Cell {
    fn default() -> Self {
        Self::empty()
    }
}

impl PartialEq for Cell {
    fn eq(&self, other: &Self) -> bool {
        self.hash == other.hash
    }
}

impl Eq for Cell {}

impl fmt::Debug for Cell {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Cell")
            .field("bit_len", &self.bit_len)
            .field("data", &hex::encode(&self.data))
            .field("refs", &self.refs.len())
            .field("hash", &hex::encode(self.hash))
            .finish()
    }
}

// The JSON form of a cell is its Bag of Cells serialization (with CRC32-C)
// encoded as base64, matching what TON tooling exchanges; hex is accepted on
// the way in as well. The wire format is the hand-rolled cell/BoC encoding
// and is never derived from serde.
#[cfg(feature = "serde")]
impl serde::Serialize for Cell {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use base64::engine::general_purpose::STANDARD;
        use base64::Engine;
        serializer.serialize_str(&STANDARD.encode(super::boc::serialize_boc(self, true)))
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Cell {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use base64::engine::general_purpose::{STANDARD, URL_SAFE};
        use base64::Engine;
        use serde::de::Error as DeError;

        let text = String::deserialize(deserializer)?;
        // Probe hex *first*: every even-length hex string is also valid
        // standard base64, so trying base64 first would decode the documented
        // hex form into garbage instead. A base64-encoded BoC always starts
        // with 't' (the magic byte 0xB5 maps to base64 index 45), so it can
        // never be mistaken for hex.
        let bytes = if is_hex(&text) {
            hex::decode(&text).map_err(|_| DeError::custom(EXPECTED_BOC_STRING))?
        } else {
            STANDARD
                .decode(&text)
                .or_else(|_| URL_SAFE.decode(&text))
                .map_err(|_| DeError::custom(EXPECTED_BOC_STRING))?
        };
        super::boc::parse_boc_single_root(&bytes).map_err(DeError::custom)
    }
}

/// Error message shared by the [`Cell`] deserializer's decoding branches.
#[cfg(feature = "serde")]
const EXPECTED_BOC_STRING: &str = "expected a base64 or hex encoded Bag of Cells";

/// Returns `true` if `text` is a non-empty, even-length string of ASCII
/// hex digits, i.e. a candidate for [`hex::decode`].
#[cfg(feature = "serde")]
fn is_hex(text: &str) -> bool {
    !text.is_empty() && text.len() % 2 == 0 && text.bytes().all(|b| b.is_ascii_hexdigit())
}

// The schema mirrors the serde form (base64 string), which a structural
// derive would not, hence the manual impl.
#[cfg(feature = "schemars")]
impl schemars::JsonSchema for Cell {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "Cell".into()
    }

    fn json_schema(generator: &mut schemars::generate::SchemaGenerator) -> schemars::Schema {
        <String>::json_schema(generator)
    }
}

/// Incremental builder for [`Cell`]: stores bits MSB-first and enforces the
/// 1023-bit / 4-reference limits with errors instead of panics.
///
/// All `store_*` methods consume and return the builder, so calls chain with
/// `?`:
///
/// ```rust
/// use omni_transaction::ton::types::CellBuilder;
///
/// let cell = CellBuilder::new()
///     .store_uint(0xDEAD_BEEF, 32)
///     .and_then(|b| b.store_bit(true))
///     .and_then(CellBuilder::build)
///     .unwrap();
/// assert_eq!(cell.bit_len(), 33);
/// ```
#[derive(Debug, Default, Clone)]
pub struct CellBuilder {
    data: Vec<u8>,
    bit_len: u16,
    refs: Vec<Arc<Cell>>,
}

impl CellBuilder {
    /// Creates an empty builder.
    pub const fn new() -> Self {
        Self {
            data: Vec::new(),
            bit_len: 0,
            refs: Vec::new(),
        }
    }

    /// Returns the number of bits stored so far.
    pub const fn bit_len(&self) -> u16 {
        self.bit_len
    }

    /// Returns the number of data bits still available (out of 1023).
    pub const fn remaining_bits(&self) -> u16 {
        MAX_CELL_BITS - self.bit_len
    }

    /// Returns the number of references stored so far.
    pub fn refs_count(&self) -> usize {
        self.refs.len()
    }

    /// Stores a single bit.
    ///
    /// # Errors
    ///
    /// Returns [`CellError::CellOverflow`] if the cell is full.
    pub fn store_bit(mut self, bit: bool) -> Result<Self, CellError> {
        if self.bit_len >= MAX_CELL_BITS {
            return Err(CellError::CellOverflow);
        }
        self.push_bit(bit);
        Ok(self)
    }

    /// Appends one bit without checking the capacity; callers must have
    /// verified that it fits.
    fn push_bit(&mut self, bit: bool) {
        if self.bit_len % 8 == 0 {
            self.data.push(0);
        }
        if bit {
            let i = usize::from(self.bit_len);
            self.data[i / 8] |= 1 << (7 - i % 8);
        }
        self.bit_len += 1;
    }

    /// Reserves room for `bits` more bits so the per-bit loops below do not
    /// grow the buffer one byte at a time.
    fn reserve_bits(&mut self, bits: u16) {
        let needed = usize::from(self.bit_len + bits).div_ceil(8);
        self.data.reserve(needed.saturating_sub(self.data.len()));
    }

    /// Appends the `bits` lowest bits of `value` (big-endian) without
    /// checking the capacity; callers must have verified that they fit.
    ///
    /// When both the builder and the value are byte-aligned the bytes are
    /// appended whole, which is bit-for-bit identical to the per-bit path.
    fn append_uint(&mut self, value: u128, bits: u16) {
        if self.bit_len % 8 == 0 && bits % 8 == 0 {
            let be = value.to_be_bytes();
            self.data
                .extend_from_slice(&be[be.len() - usize::from(bits / 8)..]);
            self.bit_len += bits;
            return;
        }
        self.reserve_bits(bits);
        for i in (0..bits).rev() {
            self.push_bit(value >> i & 1 == 1);
        }
    }

    /// Appends the first `bits` bits of `data` (MSB-first) without checking
    /// the capacity; callers must have verified that they fit and that
    /// `data` holds at least `bits` bits.
    ///
    /// When both the builder and the length are byte-aligned the bytes are
    /// appended whole, which is bit-for-bit identical to the per-bit path.
    fn append_bits(&mut self, data: &[u8], bits: u16) {
        if self.bit_len % 8 == 0 && bits % 8 == 0 {
            self.data.extend_from_slice(&data[..usize::from(bits / 8)]);
            self.bit_len += bits;
            return;
        }
        self.reserve_bits(bits);
        for i in 0..usize::from(bits) {
            self.push_bit(data[i / 8] >> (7 - i % 8) & 1 == 1);
        }
    }

    /// Stores `bits` bits (at most 128) of `value`, big-endian.
    ///
    /// # Errors
    ///
    /// Returns [`CellError::ValueOutOfRange`] if `value` does not fit in
    /// `bits` bits (or `bits > 128`), and [`CellError::CellOverflow`] if the
    /// cell is full.
    pub fn store_uint(mut self, value: u128, bits: u16) -> Result<Self, CellError> {
        if bits > 128 || (bits < 128 && value >> bits != 0) {
            return Err(CellError::ValueOutOfRange);
        }
        if bits > self.remaining_bits() {
            return Err(CellError::CellOverflow);
        }
        self.append_uint(value, bits);
        Ok(self)
    }

    /// Stores an 8-bit unsigned integer.
    ///
    /// # Errors
    ///
    /// Returns [`CellError::CellOverflow`] if the cell is full.
    pub fn store_u8(self, value: u8) -> Result<Self, CellError> {
        self.store_uint(value.into(), 8)
    }

    /// Stores a 32-bit unsigned integer, big-endian.
    ///
    /// # Errors
    ///
    /// Returns [`CellError::CellOverflow`] if the cell is full.
    pub fn store_u32(self, value: u32) -> Result<Self, CellError> {
        self.store_uint(value.into(), 32)
    }

    /// Stores a 64-bit unsigned integer, big-endian.
    ///
    /// # Errors
    ///
    /// Returns [`CellError::CellOverflow`] if the cell is full.
    pub fn store_u64(self, value: u64) -> Result<Self, CellError> {
        self.store_uint(value.into(), 64)
    }

    /// Stores whole bytes.
    ///
    /// # Errors
    ///
    /// Returns [`CellError::CellOverflow`] if the cell is full.
    pub fn store_slice(mut self, bytes: &[u8]) -> Result<Self, CellError> {
        // Saturating conversion: anything past 1023 bits overflows the cell
        // anyway, so the exact value does not matter.
        let bits = u16::try_from(bytes.len().saturating_mul(8)).unwrap_or(u16::MAX);
        if bits > self.remaining_bits() {
            return Err(CellError::CellOverflow);
        }
        self.append_bits(bytes, bits);
        Ok(self)
    }

    /// Stores the first `bit_len` bits of `data` (MSB-first).
    ///
    /// # Errors
    ///
    /// Returns [`CellError::CellOverflow`] if the cell is full, and
    /// [`CellError::ValueOutOfRange`] if `data` is shorter than `bit_len`
    /// bits.
    pub fn store_bits(mut self, data: &[u8], bit_len: usize) -> Result<Self, CellError> {
        if data.len() < bit_len.div_ceil(8) {
            return Err(CellError::ValueOutOfRange);
        }
        // Saturating conversion: anything past 1023 bits overflows the cell
        // anyway, so the exact value does not matter.
        let bits = u16::try_from(bit_len).unwrap_or(u16::MAX);
        if bits > self.remaining_bits() {
            return Err(CellError::CellOverflow);
        }
        self.append_bits(data, bits);
        Ok(self)
    }

    /// Stores a reference to a child cell.
    ///
    /// # Errors
    ///
    /// Returns [`CellError::TooManyRefs`] if 4 references are already stored
    /// and [`CellError::DepthOverflow`] if the child already has the maximum
    /// depth.
    pub fn store_ref(mut self, cell: Cell) -> Result<Self, CellError> {
        if self.refs.len() >= MAX_CELL_REFS {
            return Err(CellError::TooManyRefs);
        }
        if cell.depth() >= MAX_CELL_DEPTH {
            return Err(CellError::DepthOverflow);
        }
        self.refs.push(Arc::new(cell));
        Ok(self)
    }

    /// Stores a `Grams` / `VarUInteger 16` coin amount: a 4-bit byte length
    /// followed by that many value bytes (minimal length; zero is `0000`).
    ///
    /// # Errors
    ///
    /// Returns [`CellError::ValueOutOfRange`] if the amount needs more than
    /// 15 bytes (the `VarUInteger 16` maximum) and
    /// [`CellError::CellOverflow`] if the cell is full.
    pub fn store_coins(self, coins: Coins) -> Result<Self, CellError> {
        let byte_len = coins.byte_len();
        if byte_len > 15 {
            return Err(CellError::ValueOutOfRange);
        }
        self.store_uint(byte_len.into(), 4)?
            .store_uint(coins.0, u16::from(byte_len) * 8)
    }

    /// Stores an internal address in `addr_std$10` form: tag `10`, no
    /// anycast, 8-bit workchain, 256-bit account hash (267 bits total).
    ///
    /// # Errors
    ///
    /// Returns [`CellError::CellOverflow`] if the cell is full.
    pub fn store_address(self, address: &TonAddress) -> Result<Self, CellError> {
        self.store_uint(0b10, 2)?
            .store_bit(false)?
            .store_uint((address.workchain as u8).into(), 8)?
            .store_slice(&address.hash)
    }

    /// Appends all data bits and references of `cell` inline (not as a
    /// reference).
    ///
    /// # Errors
    ///
    /// Returns [`CellError::CellOverflow`] or [`CellError::TooManyRefs`] if
    /// the combined content does not fit.
    pub fn store_cell(mut self, cell: &Cell) -> Result<Self, CellError> {
        if self.refs.len() + cell.refs().len() > MAX_CELL_REFS {
            return Err(CellError::TooManyRefs);
        }
        self = self.store_bits(cell.data(), usize::from(cell.bit_len()))?;
        self.refs.extend(cell.refs().iter().cloned());
        Ok(self)
    }

    /// Finalizes the builder into a [`Cell`], applying the completion tag
    /// (one `1` bit then `0` bits) when the bit length is not a multiple of
    /// eight.
    ///
    /// # Errors
    ///
    /// Returns [`CellError::DepthOverflow`] if the resulting tree would be
    /// deeper than 1023 (other limits are enforced while storing).
    pub fn build(mut self) -> Result<Cell, CellError> {
        if self.bit_len % 8 != 0 {
            let i = usize::from(self.bit_len);
            self.data[i / 8] |= 1 << (7 - i % 8);
        }
        Cell::from_parts(self.data, self.bit_len, self.refs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Spec vector: the empty cell hash equals `SHA256(0x00 0x00)`.
    #[test]
    fn test_empty_cell_hash_matches_vector() {
        assert_eq!(
            hex::encode(Cell::empty().repr_hash()),
            "96a296d224f285c67bee93c30f8a309157f0daa35dc5b87e410b78630a09cfc7"
        );
        assert_eq!(Cell::empty().depth(), 0);
        assert_eq!(Cell::default(), Cell::empty());
    }

    /// Spec vector: 7-bit cell (uint7 value 5) exercising the completion tag.
    #[test]
    fn test_seven_bit_cell_hash_and_completion_tag() {
        let cell = CellBuilder::new()
            .store_uint(5, 7)
            .unwrap()
            .build()
            .unwrap();
        assert_eq!(cell.bit_len(), 7);
        // 0000101 + completion tag '1' = 0x0B.
        assert_eq!(cell.data(), &[0x0B]);
        assert_eq!(Cell::descriptor_d2(cell.bit_len()), 0x01);
        assert_eq!(
            hex::encode(cell.repr_hash()),
            "bca917d32b52f0d854ac1a4575be15e556c78b2e04a2a2ea5f0b4fd1d97a9cd9"
        );
    }

    /// Spec vector: parent with two refs exercising depths and child hashes
    /// in the representation-hash preimage.
    #[test]
    fn test_parent_cell_hash_with_refs() {
        let leaf = CellBuilder::new()
            .store_uint(0xF, 32)
            .unwrap()
            .build()
            .unwrap();
        assert_eq!(
            hex::encode(leaf.repr_hash()),
            "57b520dbcb9d135863fc33963cde9f6db2ded1430d88056810a2c9434a3860f9"
        );
        let seven_bit = CellBuilder::new()
            .store_uint(5, 7)
            .unwrap()
            .build()
            .unwrap();
        let parent = CellBuilder::new()
            .store_uint(0xDEAD_BEEF, 32)
            .unwrap()
            .store_ref(leaf)
            .unwrap()
            .store_ref(seven_bit)
            .unwrap()
            .build()
            .unwrap();
        assert_eq!(parent.depth(), 1);
        assert_eq!(
            hex::encode(parent.repr_hash()),
            "7a8347cf3f20b0d959c04f0e02b02fe4a7b17f3c89e665f9532f410e8d0fde58"
        );
    }

    #[test]
    fn test_builder_enforces_bit_capacity() {
        let mut builder = CellBuilder::new();
        for _ in 0..1023 {
            builder = builder.store_bit(true).unwrap();
        }
        assert_eq!(builder.remaining_bits(), 0);
        assert_eq!(
            builder.store_bit(true).unwrap_err(),
            CellError::CellOverflow
        );
    }

    #[test]
    fn test_builder_enforces_ref_capacity() {
        let mut builder = CellBuilder::new();
        for _ in 0..4 {
            builder = builder.store_ref(Cell::empty()).unwrap();
        }
        assert_eq!(
            builder.store_ref(Cell::empty()).unwrap_err(),
            CellError::TooManyRefs
        );
    }

    #[test]
    fn test_store_uint_rejects_out_of_range_values() {
        assert_eq!(
            CellBuilder::new().store_uint(2, 1).unwrap_err(),
            CellError::ValueOutOfRange
        );
        assert_eq!(
            CellBuilder::new().store_uint(0, 129).unwrap_err(),
            CellError::ValueOutOfRange
        );
        // Full-width value is fine.
        assert!(CellBuilder::new().store_uint(u128::MAX, 128).is_ok());
    }

    #[test]
    fn test_store_coins_encodings() {
        // Grams 0 -> '0000'.
        let zero = CellBuilder::new()
            .store_coins(Coins(0))
            .unwrap()
            .build()
            .unwrap();
        assert_eq!(zero.bit_len(), 4);
        assert_eq!(zero.data(), &[0x08]); // 0000 + completion tag.

        // 1 nanoton -> len 1, value 0x01.
        let one = CellBuilder::new()
            .store_coins(Coins(1))
            .unwrap()
            .build()
            .unwrap();
        assert_eq!(one.bit_len(), 12);
        assert_eq!(one.data(), &[0x10, 0x18]); // 0001 00000001 + tag.

        // Spec vector: 0.05 TON = 50_000_000 -> '0100' + 0x02FAF080.
        let fee = CellBuilder::new()
            .store_coins(Coins(50_000_000))
            .unwrap()
            .build()
            .unwrap();
        assert_eq!(fee.bit_len(), 36);
        assert_eq!(fee.data(), &[0x40, 0x2F, 0xAF, 0x08, 0x08]);

        // Guardrail: 15-byte max fits, 16-byte values do not.
        assert!(CellBuilder::new()
            .store_coins(Coins((1 << 120) - 1))
            .is_ok());
        assert_eq!(
            CellBuilder::new().store_coins(Coins(1 << 120)).unwrap_err(),
            CellError::ValueOutOfRange
        );
        assert_eq!(
            CellBuilder::new()
                .store_coins(Coins(u128::MAX))
                .unwrap_err(),
            CellError::ValueOutOfRange
        );
    }

    /// Reference implementation of the store paths: one `store_bit` per bit,
    /// which is what the builder did before the byte-aligned fast paths.
    fn reference_bits(prefix: &[bool], bits: &[bool]) -> Cell {
        let mut builder = CellBuilder::new();
        for &bit in prefix.iter().chain(bits) {
            builder = builder.store_bit(bit).unwrap();
        }
        builder.build().unwrap()
    }

    fn bits_of(data: &[u8], bit_len: usize) -> Vec<bool> {
        (0..bit_len)
            .map(|i| data[i / 8] >> (7 - i % 8) & 1 == 1)
            .collect()
    }

    /// The byte-aligned fast paths of `store_slice` / `store_bits` /
    /// `store_uint` / `store_cell` must produce exactly the bits the per-bit
    /// implementation produces, at every starting offset.
    #[test]
    fn test_byte_aligned_fast_paths_match_bit_by_bit() {
        let payload: Vec<u8> = (0u8..37).map(|i| i.wrapping_mul(37) ^ 0xA5).collect();
        for prefix_len in 0..=17usize {
            let prefix: Vec<bool> = (0..prefix_len).map(|i| i % 3 == 0).collect();
            let mut start = CellBuilder::new();
            for &bit in &prefix {
                start = start.store_bit(bit).unwrap();
            }

            // store_slice (whole bytes).
            let expected = reference_bits(&prefix, &bits_of(&payload, payload.len() * 8));
            let actual = start
                .clone()
                .store_slice(&payload)
                .unwrap()
                .build()
                .unwrap();
            assert_eq!(actual.data(), expected.data(), "store_slice @ {prefix_len}");
            assert_eq!(actual.bit_len(), expected.bit_len());
            assert_eq!(actual.repr_hash(), expected.repr_hash());

            // store_bits, aligned and unaligned lengths.
            for bit_len in [0usize, 1, 7, 8, 9, 63, 64, 100, 293] {
                let expected = reference_bits(&prefix, &bits_of(&payload, bit_len));
                let actual = start
                    .clone()
                    .store_bits(&payload, bit_len)
                    .unwrap()
                    .build()
                    .unwrap();
                assert_eq!(
                    actual.data(),
                    expected.data(),
                    "store_bits({bit_len}) @ {prefix_len}"
                );
                assert_eq!(actual.bit_len(), expected.bit_len());
            }

            // store_uint at every width that fits, aligned and unaligned.
            for bits in [0u16, 1, 4, 8, 15, 16, 32, 64, 127, 128] {
                let raw = u128::from_be_bytes(payload[..16].try_into().unwrap());
                let value = if bits == 0 {
                    0
                } else {
                    raw >> (128 - u32::from(bits))
                };
                let value_bits: Vec<bool> = (0..bits)
                    .rev()
                    .map(|i| value >> i & 1 == 1)
                    .collect::<Vec<_>>();
                let expected = reference_bits(&prefix, &value_bits);
                let actual = start
                    .clone()
                    .store_uint(value, bits)
                    .unwrap()
                    .build()
                    .unwrap();
                assert_eq!(
                    actual.data(),
                    expected.data(),
                    "store_uint({value}, {bits}) @ {prefix_len}"
                );
                assert_eq!(actual.bit_len(), expected.bit_len());
            }

            // store_cell inlines through the same path.
            let inner = CellBuilder::new()
                .store_slice(&payload[..11])
                .unwrap()
                .store_bit(true)
                .unwrap()
                .build()
                .unwrap();
            let expected = reference_bits(&prefix, &bits_of(inner.data(), 89));
            let actual = start.clone().store_cell(&inner).unwrap().build().unwrap();
            assert_eq!(actual.data(), expected.data(), "store_cell @ {prefix_len}");
            assert_eq!(actual.bit_len(), expected.bit_len());
        }
    }

    /// The capacity and range errors of the fast paths match the per-bit
    /// behaviour: `ValueOutOfRange` wins over `CellOverflow` in `store_bits`,
    /// and an over-long payload never partially fills the cell.
    #[test]
    fn test_fast_paths_report_the_same_errors() {
        let full = |bits: u16| {
            let mut builder = CellBuilder::new();
            for _ in 0..bits {
                builder = builder.store_bit(false).unwrap();
            }
            builder
        };
        assert_eq!(
            full(1016).store_slice(&[0u8; 2]).unwrap_err(),
            CellError::CellOverflow
        );
        assert!(full(1015).store_slice(&[0u8; 1]).is_ok());
        assert_eq!(
            full(0).store_slice(&[0u8; 128]).unwrap_err(),
            CellError::CellOverflow
        );
        assert_eq!(
            full(0).store_bits(&[0u8; 4], 33).unwrap_err(),
            CellError::ValueOutOfRange
        );
        // Short data *and* an over-capacity length: range error wins.
        assert_eq!(
            full(0).store_bits(&[0u8; 4], 5000).unwrap_err(),
            CellError::ValueOutOfRange
        );
        assert_eq!(
            full(0).store_bits(&[0u8; 1024], 1024).unwrap_err(),
            CellError::CellOverflow
        );
        assert_eq!(
            full(1000).store_uint(0, 24).unwrap_err(),
            CellError::CellOverflow
        );
        assert_eq!(
            full(1000).store_uint(1 << 30, 24).unwrap_err(),
            CellError::ValueOutOfRange
        );
    }

    #[test]
    fn test_store_cell_inlines_bits_and_refs() {
        let inner = CellBuilder::new()
            .store_uint(0b101, 3)
            .unwrap()
            .store_ref(Cell::empty())
            .unwrap()
            .build()
            .unwrap();
        let outer = CellBuilder::new()
            .store_bit(true)
            .unwrap()
            .store_cell(&inner)
            .unwrap()
            .build()
            .unwrap();
        assert_eq!(outer.bit_len(), 4);
        assert_eq!(outer.refs().len(), 1);
        // '1' + '101' + completion tag '1' = 0xD8.
        assert_eq!(outer.data(), &[0xD8]);
    }

    #[test]
    fn test_store_address_layout() {
        let address = TonAddress::new(0, [0xAB; 32]);
        let cell = CellBuilder::new()
            .store_address(&address)
            .unwrap()
            .build()
            .unwrap();
        assert_eq!(cell.bit_len(), 267);
    }

    /// The documented hex form and the base64 form of the same BoC must
    /// deserialize to the same cell (hex is probed first because every
    /// even-length hex string also decodes as standard base64).
    #[cfg(feature = "serde_json")]
    #[test]
    fn test_cell_deserializes_hex_and_base64() {
        use base64::engine::general_purpose::{STANDARD, URL_SAFE};
        use base64::Engine;

        for cell in [
            Cell::empty(),
            CellBuilder::new()
                .store_uint(0xDEAD_BEEF, 32)
                .unwrap()
                .store_ref(Cell::empty())
                .unwrap()
                .build()
                .unwrap(),
        ] {
            for with_crc in [false, true] {
                let boc = super::super::boc::serialize_boc(&cell, with_crc);
                let hex_json = format!("\"{}\"", hex::encode(&boc));
                let base64_json = format!("\"{}\"", STANDARD.encode(&boc));
                let url_safe_json = format!("\"{}\"", URL_SAFE.encode(&boc));
                for json in [&hex_json, &base64_json, &url_safe_json] {
                    let parsed: Cell = serde_json::from_str(json).unwrap();
                    assert_eq!(parsed, cell, "failed for {json}");
                }
            }
        }
        // The empty-cell hex vector from the BoC spec tests.
        let parsed: Cell = serde_json::from_str("\"b5ee9c72010101010002000000\"").unwrap();
        assert_eq!(parsed, Cell::empty());
        // Neither hex nor base64.
        assert!(serde_json::from_str::<Cell>("\"not a bag of cells!\"").is_err());
    }

    #[cfg(feature = "serde_json")]
    #[test]
    fn test_cell_serde_round_trip() {
        let cell = CellBuilder::new()
            .store_uint(0xDEAD_BEEF, 32)
            .unwrap()
            .store_ref(Cell::empty())
            .unwrap()
            .build()
            .unwrap();
        let json = serde_json::to_string(&cell).unwrap();
        let back: Cell = serde_json::from_str(&json).unwrap();
        assert_eq!(back, cell);
    }
}
