//! TON transaction: wallet body encoding, signing payload and the signed
//! external message.
use core::fmt;

#[cfg(feature = "schemars")]
use schemars::JsonSchema;
#[cfg(feature = "serde")]
use serde::{Deserialize, Deserializer, Serialize};

use super::constants::{
    ACTION_SEND_MSG_OPCODE, SEND_MODE_IGNORE_ERRORS, V5R1_SIGNED_EXTERNAL_OPCODE,
};
use super::types::{
    serialize_boc, Cell, CellBuilder, CellError, InternalMessage, TonAddress, WalletVersion,
    MAX_CELL_REFS,
};
use super::utils::derive_wallet_address;
use crate::constants::{ED25519_PUBLIC_KEY_LENGTH, ED25519_SIGNATURE_LENGTH};

/// Maximum number of internal messages a v4r2 wallet transaction can carry
/// (one cell reference per message).
pub const V4R2_MAX_MESSAGES: usize = 4;
/// Maximum number of internal messages a v5r1 wallet transaction can carry
/// (`OutList` actions).
pub const V5R1_MAX_MESSAGES: usize = 255;

/// A TON wallet transaction: one or more internal transfers wrapped in a
/// signed external-in message for a v4r2 or v5r1 (W5) wallet contract.
///
/// # Encoding entry points
///
/// - [`Self::build_for_signing`] — the **final 32-byte** representation hash
///   of the unsigned wallet body. TON wallets verify ed25519 signatures over
///   this cell hash, so sign these bytes directly (plain RFC 8032, no extra
///   hashing) — e.g. with NEAR MPC eddsa.
/// - [`Self::build_with_signature`] — the Bag of Cells bytes of the full
///   signed external message, ready to broadcast (e.g. base64-encoded to
///   toncenter `sendBoc`).
///
/// ###### Example:
///
/// ```rust
/// use omni_transaction::ton::types::{Coins, InternalMessage, TonAddress, WalletVersion};
/// use omni_transaction::ton::TonTransaction;
///
/// let public_key: [u8; 32] =
///     hex::decode("31debe55d37c722768b137131caa6087080b2e0b60b94bd785d14575cfa498bc")
///         .unwrap()
///         .try_into()
///         .unwrap();
/// let dest: TonAddress = "EQCD39VS5jcptHL8vMjEXrzGaRcCVYto7HUn4bpAOg8xqB2N"
///     .parse()
///     .unwrap();
///
/// let tx = TonTransaction {
///     wallet_version: WalletVersion::V5R1,
///     workchain: 0,
///     public_key,
///     wallet_id: WalletVersion::V5R1.default_wallet_id(0),
///     valid_until: 1735689600,
///     seqno: 1,
///     messages: vec![InternalMessage::new(dest, Coins::from_nano(50_000_000))],
///     deploy: false,
/// };
///
/// // 32-byte payload for the ed25519 signer.
/// let payload = tx.build_for_signing();
/// assert_eq!(
///     hex::encode(&payload),
///     "0b45a74ddb8c4bd8f9a504b255869daad1bba81a1bd03351ea5ca1a3c47a2416"
/// );
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct TonTransaction {
    /// Wallet contract version (v5r1 is the current standard).
    #[cfg_attr(feature = "serde", serde(default))]
    pub wallet_version: WalletVersion,
    /// Workchain of the wallet (0 = basechain).
    #[cfg_attr(feature = "serde", serde(default))]
    pub workchain: i8,
    /// The ed25519 public key controlling the wallet; determines the wallet
    /// address together with the version, workchain and wallet id.
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_public_key"))]
    pub public_key: [u8; ED25519_PUBLIC_KEY_LENGTH],
    /// v4r2 `subwallet_id` or v5r1 wallet id
    /// (see [`crate::ton::types::v5r1_wallet_id`]).
    pub wallet_id: u32,
    /// Expiration as a unix timestamp in seconds; the network rejects the
    /// message after this time.
    pub valid_until: u32,
    /// Wallet sequence number (fetch with the wallet's `seqno` get-method;
    /// 0 for a not-yet-deployed wallet).
    pub seqno: u32,
    /// The internal transfers to send: at most [`V4R2_MAX_MESSAGES`] /
    /// [`V5R1_MAX_MESSAGES`] depending on the version.
    pub messages: Vec<InternalMessage>,
    /// Whether to attach the wallet `StateInit` (code + initial data),
    /// required for the first transaction of a wallet (`seqno` 0).
    #[cfg_attr(feature = "serde", serde(default))]
    pub deploy: bool,
}

impl TonTransaction {
    /// Returns the wallet address this transaction is sent to (derived from
    /// the version, workchain, wallet id and public key).
    pub fn wallet_address(&self) -> TonAddress {
        derive_wallet_address(
            self.wallet_version,
            self.workchain,
            self.wallet_id,
            &self.public_key,
        )
    }

    /// Returns the **final 32-byte signing payload**: the representation
    /// hash of the unsigned wallet body cell. Sign it directly with ed25519
    /// (plain RFC 8032; do **not** hash again) — TON contracts verify
    /// `check_signature(cell_hash, signature, public_key)`.
    ///
    /// # Panics
    ///
    /// Panics if the transaction is malformed (too many messages for the
    /// wallet version, or a transfer that cannot be encoded).
    pub fn build_for_signing(&self) -> Vec<u8> {
        self.unsigned_body_cell().repr_hash().to_vec()
    }

    /// Returns the Bag of Cells bytes (with CRC32-C, as standard tooling
    /// broadcasts) of the signed external message: `ext_in_msg_info$10`
    /// header, optional `StateInit` when [`Self::deploy`] is set, and the
    /// signed wallet body.
    ///
    /// Base64-encode these bytes for the toncenter `sendBoc` /
    /// `/api/v3/message` endpoints (or use
    /// [`Self::build_with_signature_base64`]).
    ///
    /// # Panics
    ///
    /// Panics if the transaction is malformed (too many messages for the
    /// wallet version, or a transfer that cannot be encoded).
    pub fn build_with_signature(&self, signature: [u8; ED25519_SIGNATURE_LENGTH]) -> Vec<u8> {
        let body = self
            .body_cell(Some(&signature))
            .expect("signed body encoding follows the unsigned body");
        let external = self
            .try_external_message(&body)
            .expect("external message encoding is infallible for valid bodies");
        serialize_boc(&external, true)
    }

    /// Convenience wrapper around [`Self::build_with_signature`] returning
    /// the base64 string toncenter's `sendBoc` endpoint expects.
    pub fn build_with_signature_base64(&self, signature: [u8; ED25519_SIGNATURE_LENGTH]) -> String {
        use base64::engine::general_purpose::STANDARD;
        use base64::Engine;
        STANDARD.encode(self.build_with_signature(signature))
    }

    /// Returns the unsigned wallet body cell whose representation hash is
    /// the signing payload.
    ///
    /// # Panics
    ///
    /// Panics if the transaction is malformed (too many messages for the
    /// wallet version, or a transfer that cannot be encoded).
    pub fn unsigned_body_cell(&self) -> Cell {
        self.body_cell(None)
            .expect("wallet body encoding failed; check message count and sizes")
    }

    /// Encodes the wallet body, optionally with the signature placed where
    /// the wallet version expects it (v4r2: prepended; v5r1: appended).
    fn body_cell(&self, signature: Option<&[u8; 64]>) -> Result<Cell, CellError> {
        match self.wallet_version {
            WalletVersion::V4R2 => {
                assert!(
                    self.messages.len() <= V4R2_MAX_MESSAGES,
                    "wallet v4r2 supports at most {V4R2_MAX_MESSAGES} messages per transaction"
                );
                let mut builder = CellBuilder::new();
                if let Some(signature) = signature {
                    builder = builder.store_slice(signature)?;
                }
                builder = builder
                    .store_u32(self.wallet_id)?
                    .store_u32(self.valid_until)?
                    .store_u32(self.seqno)?
                    .store_u8(0)?; // op: simple send
                for message in &self.messages {
                    builder = builder
                        .store_u8(message.mode)?
                        .store_ref(message.to_cell()?)?;
                }
                builder.build()
            }
            WalletVersion::V5R1 => {
                assert!(
                    self.messages.len() <= V5R1_MAX_MESSAGES,
                    "wallet v5r1 supports at most {V5R1_MAX_MESSAGES} messages per transaction"
                );
                let mut builder = CellBuilder::new()
                    .store_u32(V5R1_SIGNED_EXTERNAL_OPCODE)?
                    .store_u32(self.wallet_id)?
                    .store_u32(self.valid_until)?
                    .store_u32(self.seqno)?
                    .store_bit(true)? // out actions present
                    .store_ref(self.out_list()?)?
                    .store_bit(false)?; // no extended actions
                if let Some(signature) = signature {
                    builder = builder.store_slice(signature)?;
                }
                builder.build()
            }
        }
    }

    /// Builds the v5r1 `OutList`: a linked list with one cell per
    /// `action_send_msg`, previous list first, then opcode, mode and the
    /// message reference. The first message ends up innermost.
    fn out_list(&self) -> Result<Cell, CellError> {
        let mut list = Cell::empty();
        for message in &self.messages {
            list = CellBuilder::new()
                .store_ref(list)?
                .store_u32(ACTION_SEND_MSG_OPCODE)?
                // Externally-authorized v5r1 requests must ignore errors.
                .store_u8(message.mode | SEND_MODE_IGNORE_ERRORS)?
                .store_ref(message.to_cell()?)?
                .build()?;
        }
        Ok(list)
    }

    /// Wraps the signed body in the external-in message: header (277 bits),
    /// optional `StateInit`, then the body — each inlined exactly when
    /// reference tooling inlines them, so the bytes match across
    /// implementations.
    fn try_external_message(&self, body: &Cell) -> Result<Cell, CellError> {
        let mut builder = CellBuilder::new()
            .store_uint(0b10, 2)? // ext_in_msg_info$10
            .store_uint(0b00, 2)? // src: addr_none$00
            .store_address(&self.wallet_address())? // dest: addr_std$10
            .store_uint(0, 4)?; // import_fee: Grams 0
        builder = if self.deploy {
            let init = self
                .wallet_version
                .state_init_cell(self.wallet_id, &self.public_key);
            builder = builder.store_bit(true)?; // init present
            if usize::from(builder.remaining_bits())
                >= 2 + usize::from(init.bit_len()) + usize::from(body.bit_len())
            {
                builder.store_bit(false)?.store_cell(&init)?
            } else {
                builder.store_bit(true)?.store_ref(init)?
            }
        } else {
            builder.store_bit(false)? // no init
        };
        // One spare bit for the Either tag, per the reference inlining rule.
        builder = if usize::from(builder.remaining_bits()) > usize::from(body.bit_len())
            && builder.refs_count() + body.refs().len() <= MAX_CELL_REFS
        {
            builder.store_bit(false)?.store_cell(body)?
        } else {
            builder.store_bit(true)?.store_ref(body.clone())?
        };
        builder.build()
    }

    /// Parses a transaction from its JSON representation.
    ///
    /// # Errors
    ///
    /// Returns a [`serde_json::Error`] if the JSON is malformed.
    #[cfg(feature = "serde_json")]
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

impl fmt::Display for TonTransaction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:?} wallet tx (seqno {}, {} message(s))",
            self.wallet_version,
            self.seqno,
            self.messages.len()
        )
    }
}

/// Accepts the public key as a 32-byte array or a hex string (with or
/// without `0x` prefix).
#[cfg(feature = "serde")]
fn deserialize_public_key<'de, D>(deserializer: D) -> Result<[u8; 32], D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error as DeError;

    struct KeyVisitor;

    impl<'de> serde::de::Visitor<'de> for KeyVisitor {
        type Value = [u8; 32];

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a 32-byte array or a 64-character hex string")
        }

        fn visit_str<E: DeError>(self, v: &str) -> Result<Self::Value, E> {
            let v = v.strip_prefix("0x").unwrap_or(v);
            let mut key = [0u8; 32];
            hex::decode_to_slice(v, &mut key)
                .map_err(|_| DeError::custom("invalid public key hex"))?;
            Ok(key)
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::SeqAccess<'de>,
        {
            let mut key = [0u8; 32];
            for (i, byte) in key.iter_mut().enumerate() {
                *byte = seq
                    .next_element::<u8>()?
                    .ok_or_else(|| DeError::invalid_length(i, &"32 bytes"))?;
            }
            if seq.next_element::<u8>()?.is_some() {
                return Err(DeError::custom("expected exactly 32 bytes"));
            }
            Ok(key)
        }
    }

    deserializer.deserialize_any(KeyVisitor)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ton::types::Coins;

    const VALID_UNTIL: u32 = 1_735_689_600;
    const V4_SIGNATURE: &str = "4afc182e0f399fbed7a305589e8ba57fb568135def13ead61fce81200d8cfa99f0e93af3c47974733a05ccf07075e954d9e23dcde46bb52885cb88dcb9840f09";

    fn vector_pubkey() -> [u8; 32] {
        hex::decode("31debe55d37c722768b137131caa6087080b2e0b60b94bd785d14575cfa498bc")
            .unwrap()
            .try_into()
            .unwrap()
    }

    fn vector_transfer() -> InternalMessage {
        let dest: TonAddress = "0:83dfd552e63729b472fcbcc8c45ebcc6691702558b68ec7527e1ba403a0f31a8"
            .parse()
            .unwrap();
        InternalMessage::new(dest, Coins::from_nano(50_000_000))
    }

    fn v4_transaction() -> TonTransaction {
        let mut message = vector_transfer();
        message.mode = 1;
        TonTransaction {
            wallet_version: WalletVersion::V4R2,
            workchain: 0,
            public_key: vector_pubkey(),
            wallet_id: WalletVersion::V4R2.default_wallet_id(0),
            valid_until: VALID_UNTIL,
            seqno: 1,
            messages: vec![message],
            deploy: false,
        }
    }

    fn v5_transaction() -> TonTransaction {
        TonTransaction {
            wallet_version: WalletVersion::V5R1,
            workchain: 0,
            public_key: vector_pubkey(),
            wallet_id: WalletVersion::V5R1.default_wallet_id(0),
            valid_until: VALID_UNTIL,
            seqno: 1,
            messages: vec![vector_transfer()],
            deploy: false,
        }
    }

    fn signature_from_hex(hex_str: &str) -> [u8; 64] {
        let mut signature = [0u8; 64];
        hex::decode_to_slice(hex_str, &mut signature).unwrap();
        signature
    }

    /// Spec vector: v4r2 wallet address, unsigned-body hash, signed-body
    /// hash and full external message BoC.
    #[test]
    fn test_v4r2_flow_matches_vectors() {
        let tx = v4_transaction();
        assert_eq!(
            tx.wallet_address().to_string(),
            "EQC6JyR14H1yKOHN1DKMo6XbBzTBXCznvZGE0rcB0fX6ZHPD"
        );
        assert_eq!(
            hex::encode(tx.build_for_signing()),
            "84d81f593253be4c06ab026b2e44e78fc5d9a98a634021fd68b9519d1e8f4b83"
        );
        let signature = signature_from_hex(V4_SIGNATURE);
        assert_eq!(
            tx.build_with_signature_base64(signature),
            "te6cckEBAgEAqgAB4YgBdE5I68D65FHDm6hlGUdLtg5pgrhZz3sjCaVuA6Pr9MgCV+DBcHnM/fa9GCrE9F0r/atAmu94n1aw/nQJAGxn1M+HSdeeI8ujmdAuZ4ODr0qmzxHubyNdqUQuXEblzCB4SU1NGLs7pCwAAAAACAAMAQBoYgBB7+qpcxuU2jl+XmRiL15jNIuBKsW0djqT8N0gHQeY1CAX14QAAAAAAAAAAAAAAAAAAMjj5qM="
        );
    }

    /// Spec vector: the same v4r2 transaction with `deploy` includes the
    /// inlined `StateInit` (embedded wallet code + initial data).
    #[test]
    fn test_v4r2_deploy_flow_matches_vector() {
        let mut tx = v4_transaction();
        tx.deploy = true;
        let signature = signature_from_hex(V4_SIGNATURE);
        assert_eq!(
            tx.build_with_signature_base64(signature),
            "te6cckECFwEAA6wAA+OIAXROSOvA+uRRw5uoZRlHS7YOaYK4Wc97IwmlbgOj6/TIEYlfgwXB5zP32vRgqxPRdK/2rQJrveJ9WsP50CQBsZ9TPh0nXniPLo5nQLmeDg69Kps8R7m8jXalELlxG5cwgeElNTRi7O6QsAAAAAAgADABFRYBFP8A9KQT9LzyyAsCAgEgAxACAUgEBwLm0AHQ0wMhcbCSXwTgItdJwSCSXwTgAtMfIYIQcGx1Z70ighBkc3RyvbCSXwXgA/pAMCD6RAHIygfL/8nQ7UTQgQFA1yH0BDBcgQEI9ApvoTGzkl8H4AXTP8glghBwbHVnupI4MOMNA4IQZHN0crqSXwbjDQUGAHgB+gD0BDD4J28iMFAKoSG+8uBQghBwbHVngx6xcIAYUATLBSbPFlj6Ahn0AMtpF8sfUmDLPyDJgED7AAYAilAEgQEI9Fkw7UTQgQFA1yDIAc8W9ADJ7VQBcrCOI4IQZHN0coMesXCAGFAFywVQA88WI/oCE8tqyx/LP8mAQPsAkl8D4gIBIAgPAgEgCQ4CAVgKCwA9sp37UTQgQFA1yH0BDACyMoHy//J0AGBAQj0Cm+hMYAIBIAwNABmtznaiaEAga5Drhf/AABmvHfaiaEAQa5DrhY/AABG4yX7UTQ1wsfgAWb0kK29qJoQICga5D6AhhHDUCAhHpJN9KZEM5pA+n/mDeBKAG3gQFImHFZ8xhAT48oMI1xgg0x/TH9MfAvgju/Jk7UTQ0x/TH9P/9ATRUUO68qFRUbryogX5AVQQZPkQ8qP4ACSkyMsfUkDLH1Iwy/9SEPQAye1U+A8B0wchwACfbFGTINdKltMH1AL7AOgw4CHAAeMAIcAC4wABwAORMOMNA6TIyx8Syx/L/xESExQAbtIH+gDU1CL5AAXIygcVy//J0Hd0gBjIywXLAiLPFlAF+gIUy2sSzMzJc/sAyEAUgQEI9FHypwIAcIEBCNcY+gDTP8hUIEeBAQj0UfKnghBub3RlcHSAGMjLBcsCUAbPFlAE+gIUy2oSyx/LP8lz+wACAGyBAQjXGPoA0z8wUiSBAQj0WfKnghBkc3RycHSAGMjLBcsCUAXPFlAD+gITy2rLHxLLP8lz+wAACvQAye1UAFEAAAAAKamjFzHevlXTfHInaLE3ExyqYIcICy4LYLlL14XRRXXPpJi8QABoYgBB7+qpcxuU2jl+XmRiL15jNIuBKsW0djqT8N0gHQeY1CAX14QAAAAAAAAAAAAAAAAAADdEcGc="
        );
    }

    /// Spec vector: v5r1 wallet address, unsigned-body (`0x7369676e`
    /// opcode) hash and full external message BoC. The user-set mode 1 is
    /// force-ORed with `IGNORE_ERRORS` to the vector's mode 3.
    #[test]
    fn test_v5r1_flow_matches_vectors() {
        let mut tx = v5_transaction();
        tx.messages[0].mode = 1; // the builder must force +2 back in
        assert_eq!(
            tx.wallet_address().to_string(),
            "EQBYzXWenm06gusNRk6wLQwM_HO7Qs5-tVDwkPXp-ixZL-0W"
        );
        assert_eq!(
            hex::encode(tx.build_for_signing()),
            "0b45a74ddb8c4bd8f9a504b255869daad1bba81a1bd03351ea5ca1a3c47a2416"
        );
        // Signature produced with the fixed test key (seed 0x17 * 32),
        // extracted from the spec's signed-body BoC (bits 130..642).
        let signature = extract_v5_signature();
        assert_eq!(
            tx.build_with_signature_base64(signature),
            "te6cckEBBAEAtwAB5YgAsZrrPTzadQXWGoydYFoYGfjndoWc/Wqh4SHr0/RYsl4Dm0s7c///+Is7pCwAAAAADWkaJkxhW/VO+P3HsjO8WONg2oHKbvGnROVSuwi4zGMomSwfBbyvGpxOAvB7/VSEDrrWDXSJ2QMoNn/6J31+uAsBAgoOw8htAwIDAAAAaGIAQe/qqXMblNo5fl5kYi9eYzSLgSrFtHY6k/DdIB0HmNQgF9eEAAAAAAAAAAAAAAAAAACThNw1"
        );
    }

    /// Extracts the 512-bit signature embedded in the spec's v5r1
    /// signed-body vector (appended after the 130 body bits).
    fn extract_v5_signature() -> [u8; 64] {
        let boc = hex::decode(
            "b5ee9c724101040100950001a17369676e7fffff116774858000000001ad2344c98c2b7ea9df1fb8f646778b1c6c1b50394dde34e89caa576117198c65132583e0b795e35389c05e0f7faa9081d75ac1ae913b206506cfff44efafd7016001020a0ec3c86d03020300000068620041efeaa9731b94da397e5e64622f5e63348b812ac5b4763a93f0dd201d0798d42017d7840000000000000000000000000000841f1b8c",
        )
        .unwrap();
        let root = crate::ton::types::parse_boc_single_root(&boc).unwrap();
        assert_eq!(root.bit_len(), 642);
        let mut signature = [0u8; 64];
        for i in 0..512 {
            let bit_index = 130 + i;
            let bit = root.data()[bit_index / 8] >> (7 - bit_index % 8) & 1;
            signature[i / 8] |= bit << (7 - i % 8);
        }
        signature
    }

    /// Spec vector: the v5r1 signed body cell (signature appended) hashes
    /// and serializes to the recorded BoC.
    #[test]
    fn test_v5r1_signed_body_matches_vector() {
        let tx = v5_transaction();
        let signature = extract_v5_signature();
        let signed_body = tx.body_cell(Some(&signature)).unwrap();
        assert_eq!(signed_body.bit_len(), 642);
        assert_eq!(
            hex::encode(signed_body.repr_hash()),
            "bf8c025e5016fef348eb773c9fc8d44d0090fe53b52d6af9462dced7567fa955"
        );
        assert_eq!(
            hex::encode(serialize_boc(&signed_body, true)),
            "b5ee9c724101040100950001a17369676e7fffff116774858000000001ad2344c98c2b7ea9df1fb8f646778b1c6c1b50394dde34e89caa576117198c65132583e0b795e35389c05e0f7faa9081d75ac1ae913b206506cfff44efafd7016001020a0ec3c86d03020300000068620041efeaa9731b94da397e5e64622f5e63348b812ac5b4763a93f0dd201d0798d42017d7840000000000000000000000000000841f1b8c"
        );
    }

    /// Spec vector: the v4r2 signed body cell (signature prepended).
    #[test]
    fn test_v4r2_signed_body_hash_matches_vector() {
        let tx = v4_transaction();
        let signature = signature_from_hex(V4_SIGNATURE);
        let signed_body = tx.body_cell(Some(&signature)).unwrap();
        assert_eq!(
            hex::encode(signed_body.repr_hash()),
            "4c3b549e6f23d609af6896d96c923ede11cf30bef8a288bb8e71f7a70533858f"
        );
    }

    #[test]
    #[should_panic(expected = "at most 4 messages")]
    fn test_v4r2_rejects_more_than_four_messages() {
        let mut tx = v4_transaction();
        tx.messages = vec![vector_transfer(); 5];
        let _ = tx.build_for_signing();
    }

    #[test]
    fn test_multi_message_bodies_encode() {
        let mut tx = v4_transaction();
        tx.messages = vec![vector_transfer(); 4];
        assert_eq!(tx.build_for_signing().len(), 32);

        let mut tx = v5_transaction();
        tx.messages = vec![vector_transfer(); 10];
        assert_eq!(tx.build_for_signing().len(), 32);
    }

    #[cfg(feature = "serde_json")]
    #[test]
    fn test_from_json() {
        let json = r#"
        {
            "wallet_version": "V5R1",
            "public_key": "31debe55d37c722768b137131caa6087080b2e0b60b94bd785d14575cfa498bc",
            "wallet_id": 2147483409,
            "valid_until": 1735689600,
            "seqno": 1,
            "messages": [
                {
                    "dest": "EQCD39VS5jcptHL8vMjEXrzGaRcCVYto7HUn4bpAOg8xqB2N",
                    "value": "50000000"
                }
            ]
        }"#;
        let tx = TonTransaction::from_json(json).unwrap();
        assert_eq!(tx, v5_transaction());
        assert_eq!(
            hex::encode(tx.build_for_signing()),
            "0b45a74ddb8c4bd8f9a504b255869daad1bba81a1bd03351ea5ca1a3c47a2416"
        );
    }

    #[cfg(feature = "serde_json")]
    #[test]
    fn test_serde_round_trip() {
        let tx = v5_transaction();
        let json = serde_json::to_string(&tx).unwrap();
        let back = TonTransaction::from_json(&json).unwrap();
        assert_eq!(back, tx);
    }
}
