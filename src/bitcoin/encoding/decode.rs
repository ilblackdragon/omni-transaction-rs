use std::io::{BufRead, Read};

use super::extensions::ReadExt;
use super::utils::VarInt;

/// Data which can be decoded in a bitcoin-consistent way.
pub trait Decodable: Sized {
    fn decode<R: BufRead + ?Sized>(reader: &mut R) -> Result<Self, std::io::Error>;
}

struct ReadBytesFromFiniteReaderOpts {
    len: usize,
    chunk_size: usize,
}

impl Decodable for Vec<u8> {
    fn decode<R: BufRead + ?Sized>(r: &mut R) -> Result<Self, std::io::Error> {
        let len = VarInt::decode(r)?.0 as usize;
        // most real-world vec of bytes data, wouldn't be larger than 128KiB
        let opts = ReadBytesFromFiniteReaderOpts {
            len,
            chunk_size: 128 * 1024,
        };
        read_bytes_from_finite_reader(r, opts)
    }
}

/// Read `opts.len` bytes from reader, where `opts.len` could potentially be malicious.
///
/// This function relies on reader being bound in amount of data
/// it returns for OOM protection. See [`Decodable::consensus_decode_from_finite_reader`].
fn read_bytes_from_finite_reader<D: Read + ?Sized>(
    d: &mut D,
    mut opts: ReadBytesFromFiniteReaderOpts,
) -> Result<Vec<u8>, std::io::Error> {
    let mut ret = vec![];

    assert_ne!(opts.chunk_size, 0);

    while opts.len > 0 {
        let chunk_start = ret.len();
        let chunk_size = core::cmp::min(opts.len, opts.chunk_size);
        let chunk_end = chunk_start + chunk_size;
        ret.resize(chunk_end, 0u8);
        d.read_slice(&mut ret[chunk_start..chunk_end])?;
        opts.len -= chunk_size;
    }

    Ok(ret)
}
