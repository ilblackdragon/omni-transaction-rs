use std::{
    fmt,
    io::{BufRead, Write},
};

use borsh::{BorshDeserialize, BorshSerialize};
use schemars::JsonSchema;
use serde::{de::MapAccess, Deserialize, Deserializer, Serialize};

use super::hash::Hash;
use super::tx_id::Txid;

use crate::bitcoin::encoding::{Decodable, Encodable};

/// A reference to a transaction output.
///
/// ### Bitcoin Core References
///
/// * [COutPoint definition](https://github.com/bitcoin/bitcoin/blob/345457b542b6a980ccfbc868af0970a6f91d1b82/src/primitives/transaction.h#L26)
#[derive(
    Debug, Copy, Clone, Eq, PartialEq, Serialize, BorshSerialize, BorshDeserialize, JsonSchema,
)]
#[serde(crate = "near_sdk::serde")]
pub struct OutPoint {
    /// The referenced transaction's txid.
    pub txid: Txid,
    /// The index of the referenced output in its transaction's vout.
    pub vout: u32,
}

impl OutPoint {
    /// The number of bytes that an outpoint contributes to the size of a transaction.
    pub const SIZE: usize = 32 + 4; // The serialized lengths of txid and vout.

    pub const fn new(txid: Txid, vout: u32) -> Self {
        Self { txid, vout }
    }

    /// Creates a "null" `OutPoint`.
    ///
    /// This value is used for coinbase transactions because they don't have any previous outputs.
    pub const fn null() -> Self {
        Self {
            txid: Txid::all_zeros(),
            vout: u32::MAX,
        }
    }

    /// Checks if an `OutPoint` is "null".
    pub fn is_null(&self) -> bool {
        *self == Self::null()
    }
}

impl Default for OutPoint {
    fn default() -> Self {
        Self::null()
    }
}

impl Encodable for OutPoint {
    fn encode<W: Write + ?Sized>(&self, w: &mut W) -> Result<usize, std::io::Error> {
        let mut len = 0;
        len += self.txid.encode(w)?;
        len += self.vout.encode(w)?;
        Ok(len)
    }
}

impl Decodable for OutPoint {
    fn decode<R: BufRead + ?Sized>(r: &mut R) -> Result<Self, std::io::Error> {
        let txid = Txid::decode(r)?;
        let vout = Decodable::decode(r)?;
        Ok(Self { txid, vout })
    }
}

impl<'de> Deserialize<'de> for OutPoint {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(OutPointVisitor)
    }
}

struct OutPointVisitor;

impl<'de> serde::de::Visitor<'de> for OutPointVisitor {
    type Value = OutPoint;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a map with txid as a hex string and vout as a number or string")
    }

    fn visit_map<V>(self, mut map: V) -> Result<OutPoint, V::Error>
    where
        V: MapAccess<'de>,
    {
        let mut txid = None;
        let mut vout = None;

        while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
                "txid" => {
                    let txid_str: String = map.next_value()?;
                    let txid_bytes = hex::decode(&txid_str).map_err(serde::de::Error::custom)?;
                    if txid_bytes.len() != 32 {
                        return Err(serde::de::Error::custom("Invalid txid length"));
                    }
                    let mut hash_bytes = [0u8; 32];
                    hash_bytes.copy_from_slice(&txid_bytes);
                    txid = Some(Txid(Hash(hash_bytes)));
                }
                "vout" => {
                    println!("vout");
                    vout = Some(
                        map.next_value::<serde_json::Value>()
                            .and_then(|vout_value| match vout_value {
                                serde_json::Value::Number(num) => num
                                    .as_u64()
                                    .map(|n| n as u32)
                                    .ok_or_else(|| serde::de::Error::custom("Invalid vout number")),
                                serde_json::Value::String(s) => s
                                    .parse::<u32>()
                                    .map_err(|_| serde::de::Error::custom("Invalid vout string")),
                                _ => Err(serde::de::Error::custom("Invalid vout type")),
                            })?,
                    );
                }
                _ => {
                    return Err(serde::de::Error::custom(format!("Unexpected key: {}", key)));
                }
            }
        }

        let txid = txid.ok_or_else(|| serde::de::Error::missing_field("txid"))?;
        let vout = vout.ok_or_else(|| serde::de::Error::missing_field("vout"))?;

        Ok(OutPoint { txid, vout })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode() {
        let outpoint = OutPoint {
            txid: Txid::all_zeros(),
            vout: u32::MAX,
        };

        let mut buf = Vec::new();
        outpoint.encode(&mut buf).unwrap();
        assert_eq!(buf.len(), OutPoint::SIZE);

        let decoded_outpoint = OutPoint::decode_from_finite_reader(&mut buf.as_slice()).unwrap();
        assert_eq!(decoded_outpoint, outpoint);
    }

    #[test]
    fn test_serde_json_outpoint() {
        let json_string = r#"{
            "txid":"bc25cc0dddd0a202c21e66521a692c0586330a9a9dcc38ccd9b4d2093037f31a",
            "vout":0
        }"#;

        let outpoint: OutPoint = serde_json::from_str(json_string).unwrap();
        println!("outpoint = {:?}", outpoint);
        assert_eq!(
            outpoint,
            OutPoint {
                txid: Txid(
                    Hash::from_hex(
                        "bc25cc0dddd0a202c21e66521a692c0586330a9a9dcc38ccd9b4d2093037f31a"
                    )
                    .unwrap()
                ),
                vout: 0
            }
        );
    }

    #[test]
    fn test_serde_json_outpoint_with_string_vout() {
        let json_string = r#"{
            "txid":"bc25cc0dddd0a202c21e66521a692c0586330a9a9dcc38ccd9b4d2093037f31a",
            "vout":"0"
        }"#;

        let outpoint: OutPoint = serde_json::from_str(json_string).unwrap();
        println!("outpoint = {:?}", outpoint);
        assert_eq!(
            outpoint,
            OutPoint {
                txid: Txid(
                    Hash::from_hex(
                        "bc25cc0dddd0a202c21e66521a692c0586330a9a9dcc38ccd9b4d2093037f31a"
                    )
                    .unwrap()
                ),
                vout: 0
            }
        );
    }
}
