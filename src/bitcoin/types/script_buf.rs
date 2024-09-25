use std::io::{BufRead, Write};

use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};

use crate::bitcoin::encoding::{encode::Encodable, Decodable};

#[derive(
    Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize, BorshSerialize, BorshDeserialize,
)]
pub struct ScriptBuf(pub Vec<u8>);

impl Encodable for ScriptBuf {
    fn encode<W: Write + ?Sized>(&self, w: &mut W) -> Result<usize, std::io::Error> {
        self.0.encode(w)
    }
}

impl Decodable for ScriptBuf {
    fn decode_from_finite_reader<R: BufRead + ?Sized>(r: &mut R) -> Result<Self, std::io::Error> {
        Ok(Self(Decodable::decode_from_finite_reader(r)?))
    }
}
