//! Bag of Cells (BoC) serialization and parsing.
//!
//! Implements the `serialized_boc#b5ee9c72` layout from
//! `ton-blockchain/ton crypto/tl/boc.tlb`: cells in topological order
//! (parents before children, identical subtrees deduplicated by
//! representation hash), minimal `size`/`off_bytes`, and an optional CRC32-C
//! checksum — the exact flavor produced by standard TON tooling.
use core::fmt;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use super::cell::{Cell, CellError, MAX_CELL_REFS};
use crate::ton::utils::crc32c;

/// The 4-byte `serialized_boc` magic prefix.
pub const BOC_MAGIC: [u8; 4] = [0xB5, 0xEE, 0x9C, 0x72];

/// Error raised while parsing a Bag of Cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BocError {
    /// The input does not start with the `b5ee9c72` magic.
    InvalidMagic,
    /// The input ended before the announced structures were read.
    UnexpectedEof,
    /// A header field has an out-of-spec value (flags, `size`, `off_bytes`,
    /// root or absent counts).
    InvalidHeader,
    /// A cell references a cell that precedes it, an out-of-range index, or
    /// carries invalid descriptors/padding.
    InvalidCell,
    /// The input contains exotic or non-level-0 cells, which this library
    /// does not support.
    UnsupportedCell,
    /// The CRC32-C checksum does not match.
    CrcMismatch,
    /// A parsed cell violates a cell invariant.
    Cell(CellError),
}

impl fmt::Display for BocError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidMagic => write!(f, "bag of cells does not start with b5ee9c72"),
            Self::UnexpectedEof => write!(f, "bag of cells is truncated"),
            Self::InvalidHeader => write!(f, "bag of cells header is invalid"),
            Self::InvalidCell => write!(f, "bag of cells contains an invalid cell"),
            Self::UnsupportedCell => {
                write!(f, "bag of cells contains exotic or non-level-0 cells")
            }
            Self::CrcMismatch => write!(f, "bag of cells CRC32-C mismatch"),
            Self::Cell(e) => write!(f, "bag of cells cell error: {e}"),
        }
    }
}

impl std::error::Error for BocError {}

impl From<CellError> for BocError {
    fn from(e: CellError) -> Self {
        Self::Cell(e)
    }
}

/// Returns all cells reachable from `root` in the order standard TON tooling
/// serializes them: every parent before its children, identical subtrees
/// (by representation hash) emitted once.
///
/// The order is a reverse post-order depth-first traversal that visits
/// references right-to-left, which equals pre-order on trees and stays a
/// valid topological order on DAGs with shared subtrees.
fn topological_order(root: &Arc<Cell>) -> Vec<Arc<Cell>> {
    fn visit(cell: &Arc<Cell>, seen: &mut BTreeSet<[u8; 32]>, post_order: &mut Vec<Arc<Cell>>) {
        if !seen.insert(cell.repr_hash()) {
            return;
        }
        for r in cell.refs().iter().rev() {
            visit(r, seen, post_order);
        }
        post_order.push(cell.clone());
    }

    let mut post_order = Vec::new();
    visit(root, &mut BTreeSet::new(), &mut post_order);
    post_order.reverse();
    post_order
}

/// Returns the minimal number of bytes (at least one) needed to hold `value`.
const fn min_bytes(value: usize) -> usize {
    let bits = usize::BITS - value.leading_zeros();
    let bytes = bits.div_ceil(8) as usize;
    if bytes == 0 {
        1
    } else {
        bytes
    }
}

/// Appends `value` as `width` big-endian bytes.
fn push_be(out: &mut Vec<u8>, value: usize, width: usize) {
    for i in (0..width).rev() {
        out.push((value >> (i * 8)) as u8);
    }
}

/// Serializes a single-root Bag of Cells (`has_idx = 0`), optionally with
/// the CRC32-C checksum (`with_crc = true` matches what standard tooling
/// broadcasts).
pub fn serialize_boc(root: &Cell, with_crc: bool) -> Vec<u8> {
    let order = topological_order(&Arc::new(root.clone()));
    let index: BTreeMap<[u8; 32], usize> = order
        .iter()
        .enumerate()
        .map(|(i, c)| (c.repr_hash(), i))
        .collect();

    let size = min_bytes(order.len());
    let mut cell_data = Vec::new();
    for cell in &order {
        cell_data.push(Cell::descriptor_d1(cell.refs()));
        cell_data.push(Cell::descriptor_d2(cell.bit_len()));
        cell_data.extend_from_slice(cell.data());
        for r in cell.refs() {
            push_be(&mut cell_data, index[&r.repr_hash()], size);
        }
    }
    let off_bytes = min_bytes(cell_data.len());

    let mut out = Vec::with_capacity(cell_data.len() + 16);
    out.extend_from_slice(&BOC_MAGIC);
    out.push(if with_crc { 0x40 } else { 0 } | size as u8);
    out.push(off_bytes as u8);
    push_be(&mut out, order.len(), size); // cells
    push_be(&mut out, 1, size); // roots
    push_be(&mut out, 0, size); // absent
    push_be(&mut out, cell_data.len(), off_bytes); // tot_cells_size
    push_be(&mut out, 0, size); // root_list: root is cell 0
    out.extend_from_slice(&cell_data);
    if with_crc {
        out.extend_from_slice(&crc32c(&out).to_le_bytes());
    }
    out
}

/// Cursor over the BoC byte stream with EOF checking.
struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8], BocError> {
        let end = self.pos.checked_add(n).ok_or(BocError::UnexpectedEof)?;
        let slice = self
            .bytes
            .get(self.pos..end)
            .ok_or(BocError::UnexpectedEof)?;
        self.pos = end;
        Ok(slice)
    }

    fn read_be(&mut self, width: usize) -> Result<usize, BocError> {
        let bytes = self.take(width)?;
        let mut value = 0usize;
        for &b in bytes {
            // `off_bytes` may be up to 8, which does not fit a 32-bit
            // `usize` (wasm32): report a header error instead of wrapping.
            value = value
                .checked_mul(256)
                .and_then(|v| v.checked_add(usize::from(b)))
                .ok_or(BocError::InvalidHeader)?;
        }
        Ok(value)
    }
}

/// Parses a Bag of Cells and returns its root cells (usually one).
///
/// Accepts the generic `serialized_boc#b5ee9c72` layout with or without
/// index and CRC32-C (the checksum is verified when present). Only ordinary
/// level-0 cells are supported.
///
/// Safe on untrusted input: every announced count is checked against the
/// bytes that remain before anything is allocated, so a hostile header
/// cannot trigger an out-of-memory abort.
///
/// # Errors
///
/// Returns a [`BocError`] describing the first structural problem found.
pub fn parse_boc(bytes: &[u8]) -> Result<Vec<Cell>, BocError> {
    let mut r = Reader::new(bytes);
    if r.take(4)? != BOC_MAGIC {
        return Err(BocError::InvalidMagic);
    }
    let flags = r.read_be(1)?;
    let has_idx = flags & 0x80 != 0;
    let has_crc = flags & 0x40 != 0;
    let has_cache_bits = flags & 0x20 != 0;
    let size = flags & 0x07;
    if has_cache_bits || flags & 0x18 != 0 || size == 0 || size > 4 {
        return Err(BocError::InvalidHeader);
    }
    let off_bytes = r.read_be(1)?;
    if off_bytes == 0 || off_bytes > 8 {
        return Err(BocError::InvalidHeader);
    }
    let cell_count = r.read_be(size)?;
    let root_count = r.read_be(size)?;
    let absent_count = r.read_be(size)?;
    let total_cells_size = r.read_be(off_bytes)?;
    if root_count == 0 || root_count > cell_count || absent_count != 0 {
        return Err(BocError::InvalidHeader);
    }
    // Every count in the header comes straight off the wire, so validate all
    // of them against the bytes that actually remain *before* allocating
    // anything: a 23-byte input announcing `cell_count = 0xFFFFFFFF` would
    // otherwise ask for a ~200 GB vector (abort on native, "capacity
    // overflow" on wasm32).
    let remaining = bytes.len() - r.pos;
    // Each cell record occupies at least its two descriptor bytes (d1, d2),
    // so the announced cell data must have room for all of them.
    let min_cells_size = cell_count.checked_mul(2).ok_or(BocError::InvalidHeader)?;
    if min_cells_size > total_cells_size {
        return Err(BocError::InvalidHeader);
    }
    // Root list, optional index, cell data and optional checksum must all fit
    // in the remaining input.
    let index_size = if has_idx {
        cell_count.checked_mul(off_bytes)
    } else {
        Some(0)
    };
    let announced_size = index_size
        .and_then(|index_size| index_size.checked_add(total_cells_size))
        .and_then(|n| n.checked_add(if has_crc { 4 } else { 0 }))
        .and_then(|n| root_count.checked_mul(size).and_then(|r| n.checked_add(r)))
        .ok_or(BocError::UnexpectedEof)?;
    if announced_size > remaining {
        return Err(BocError::UnexpectedEof);
    }
    // `cell_count <= total_cells_size / 2 <= remaining / 2` from here on, so
    // the allocations below are bounded by the input length.
    let mut root_indices = Vec::with_capacity(root_count);
    for _ in 0..root_count {
        let idx = r.read_be(size)?;
        if idx >= cell_count {
            return Err(BocError::InvalidHeader);
        }
        root_indices.push(idx);
    }
    if has_idx {
        r.take(
            cell_count
                .checked_mul(off_bytes)
                .ok_or(BocError::UnexpectedEof)?,
        )?;
    }

    // First pass: split the flat cell data into raw records.
    let cell_data_start = r.pos;
    let mut raw: Vec<(u8, u8, &[u8], Vec<usize>)> = Vec::with_capacity(cell_count);
    for i in 0..cell_count {
        let head = r.take(2)?;
        let (d1, d2) = (head[0], head[1]);
        if d1 & 0x08 != 0 || d1 & 0xE0 != 0 {
            return Err(BocError::UnsupportedCell);
        }
        let ref_count = usize::from(d1 & 0x07);
        if ref_count > MAX_CELL_REFS {
            return Err(BocError::InvalidCell);
        }
        let data = r.take(usize::from(d2.div_ceil(2)))?;
        let mut ref_indices = Vec::with_capacity(ref_count);
        for _ in 0..ref_count {
            let idx = r.read_be(size)?;
            // Standard BoC requires strict parents-before-children order.
            if idx <= i || idx >= cell_count {
                return Err(BocError::InvalidCell);
            }
            ref_indices.push(idx);
        }
        raw.push((d1, d2, data, ref_indices));
    }
    if r.pos - cell_data_start != total_cells_size {
        return Err(BocError::InvalidHeader);
    }
    if has_crc {
        let expected = u32::from_le_bytes(r.take(4)?.try_into().expect("take(4) returns 4 bytes"));
        if crc32c(&bytes[..r.pos - 4]) != expected {
            return Err(BocError::CrcMismatch);
        }
    }

    // Second pass: build cells bottom-up (children have higher indices).
    let mut cells: Vec<Option<Arc<Cell>>> = vec![None; cell_count];
    for i in (0..cell_count).rev() {
        let (_, d2, data, ref_indices) = &raw[i];
        let bit_len = decode_bit_len(*d2, data)?;
        let refs = ref_indices
            .iter()
            .map(|&idx| cells[idx].clone().expect("children are built first"))
            .collect();
        cells[i] = Some(Arc::new(Cell::from_parts(data.to_vec(), bit_len, refs)?));
    }
    Ok(root_indices
        .into_iter()
        .map(|idx| {
            cells[idx]
                .as_ref()
                .map(|c| (**c).clone())
                .expect("all cells are built")
        })
        .collect())
}

/// Parses a Bag of Cells that must contain exactly one root.
///
/// # Errors
///
/// Returns [`BocError::InvalidHeader`] when the BoC has more than one root
/// and any [`BocError`] from [`parse_boc`] otherwise.
pub fn parse_boc_single_root(bytes: &[u8]) -> Result<Cell, BocError> {
    let mut roots = parse_boc(bytes)?;
    if roots.len() != 1 {
        return Err(BocError::InvalidHeader);
    }
    Ok(roots.remove(0))
}

/// Recovers the bit length from the `d2` descriptor and the padded data:
/// even `d2` means whole bytes; odd `d2` means the last byte carries a
/// completion tag (lowest set `1` bit terminates the data bits).
fn decode_bit_len(d2: u8, data: &[u8]) -> Result<u16, BocError> {
    let full_bytes = u16::from(d2 / 2);
    if d2 % 2 == 0 {
        return Ok(full_bytes * 8);
    }
    let last = *data.last().ok_or(BocError::InvalidCell)?;
    if last == 0 {
        return Err(BocError::InvalidCell);
    }
    Ok(full_bytes * 8 + 7 - u16::from(last.trailing_zeros() as u8))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ton::types::cell::CellBuilder;

    fn tree() -> Cell {
        let leaf = CellBuilder::new()
            .store_uint(0xF, 32)
            .unwrap()
            .build()
            .unwrap();
        let seven_bit = CellBuilder::new()
            .store_uint(5, 7)
            .unwrap()
            .build()
            .unwrap();
        CellBuilder::new()
            .store_uint(0xDEAD_BEEF, 32)
            .unwrap()
            .store_ref(leaf)
            .unwrap()
            .store_ref(seven_bit)
            .unwrap()
            .build()
            .unwrap()
    }

    /// Spec vectors: BoC of the empty cell, with and without CRC32-C.
    #[test]
    fn test_serialize_empty_cell_matches_vectors() {
        assert_eq!(
            hex::encode(serialize_boc(&Cell::empty(), false)),
            "b5ee9c72010101010002000000"
        );
        assert_eq!(
            hex::encode(serialize_boc(&Cell::empty(), true)),
            "b5ee9c724101010100020000004cacb9cd"
        );
    }

    /// Spec vectors: BoC of the 3-cell tree pins the topological order, ref
    /// indices and CRC placement.
    #[test]
    fn test_serialize_tree_matches_vectors() {
        assert_eq!(
            hex::encode(serialize_boc(&tree(), false)),
            "b5ee9c72010103010011000208deadbeef010200080000000f00010b"
        );
        assert_eq!(
            hex::encode(serialize_boc(&tree(), true)),
            "b5ee9c72410103010011000208deadbeef010200080000000f00010bbe173f95"
        );
    }

    #[test]
    fn test_parse_round_trips() {
        for cell in [Cell::empty(), tree()] {
            for with_crc in [false, true] {
                let boc = serialize_boc(&cell, with_crc);
                let parsed = parse_boc_single_root(&boc).unwrap();
                assert_eq!(parsed.repr_hash(), cell.repr_hash());
                assert_eq!(serialize_boc(&parsed, with_crc), boc);
            }
        }
    }

    #[test]
    fn test_parse_deduplicates_shared_subtrees() {
        // Two identical children serialize as one cell and parse back shared.
        let leaf = CellBuilder::new().store_u8(7).unwrap().build().unwrap();
        let parent = CellBuilder::new()
            .store_ref(leaf.clone())
            .unwrap()
            .store_ref(leaf)
            .unwrap()
            .build()
            .unwrap();
        let boc = serialize_boc(&parent, false);
        let parsed = parse_boc_single_root(&boc).unwrap();
        assert_eq!(parsed.repr_hash(), parent.repr_hash());
        assert_eq!(parsed.refs()[0], parsed.refs()[1]);
        // 2 unique cells: parent + one shared leaf.
        assert_eq!(boc[6], 2);
    }

    /// A hostile 23-byte header announcing `cell_count = 0xFFFFFFFF` must be
    /// rejected from the header alone: allocating for it would abort the
    /// process (or panic with "capacity overflow" on wasm32).
    #[test]
    fn test_parse_rejects_oversized_cell_count_without_allocating() {
        // magic | flags(size=4) | off_bytes=1 | cells=0xFFFFFFFF | roots=1 |
        // absent=0 | tot_cells_size=0 | root_list=[0]
        let hostile = hex::decode("b5ee9c720401ffffffff00000001000000000000000000").unwrap();
        assert_eq!(hostile.len(), 23);
        assert_eq!(parse_boc(&hostile).unwrap_err(), BocError::InvalidHeader);
        assert_eq!(
            parse_boc_single_root(&hostile).unwrap_err(),
            BocError::InvalidHeader
        );
        // The same header with a huge `tot_cells_size` (so the two-bytes-per-cell
        // check passes) is rejected as truncated instead.
        let hostile = hex::decode("b5ee9c7204047fffffff0000000100000000fffffffe00000000").unwrap();
        assert_eq!(parse_boc(&hostile).unwrap_err(), BocError::UnexpectedEof);
        // Legitimate BoCs keep parsing.
        for cell in [Cell::empty(), tree()] {
            for with_crc in [false, true] {
                let boc = serialize_boc(&cell, with_crc);
                assert_eq!(
                    parse_boc_single_root(&boc).unwrap().repr_hash(),
                    cell.repr_hash()
                );
            }
        }
    }

    /// A `tot_cells_size` larger than the input is rejected before the cell
    /// data is read.
    #[test]
    fn test_parse_rejects_cells_size_larger_than_input() {
        let mut boc = serialize_boc(&tree(), false);
        assert_eq!(boc[9], 0x11); // tot_cells_size of the pinned vector
        boc[9] = 0xFF;
        assert_eq!(parse_boc(&boc).unwrap_err(), BocError::UnexpectedEof);
        // A `cell_count` inconsistent with `tot_cells_size` (fewer than two
        // bytes per cell) is a header error.
        let mut boc = serialize_boc(&tree(), false);
        assert_eq!(boc[6], 0x03); // cell_count
        boc[6] = 0x0A;
        assert_eq!(parse_boc(&boc).unwrap_err(), BocError::InvalidHeader);
    }

    #[test]
    fn test_parse_rejects_bad_input() {
        assert_eq!(parse_boc(&[]).unwrap_err(), BocError::UnexpectedEof);
        assert_eq!(
            parse_boc(&hex::decode("deadbeef010101010002000000").unwrap()).unwrap_err(),
            BocError::InvalidMagic
        );
        // Flip a payload bit (inside 0xDEADBEEF, leaving the structure
        // intact) in a checksummed BoC -> CRC mismatch.
        let mut boc = serialize_boc(&tree(), true);
        assert_eq!(boc[14], 0xAD);
        boc[14] ^= 1;
        assert_eq!(parse_boc(&boc).unwrap_err(), BocError::CrcMismatch);
        // Truncated input.
        let boc = serialize_boc(&tree(), false);
        assert_eq!(
            parse_boc(&boc[..boc.len() - 2]).unwrap_err(),
            BocError::UnexpectedEof
        );
    }
}
