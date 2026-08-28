//! Starknet INVOKE v3 transaction.
//!
//! Starknet has no byte-serialized wire transaction: the payload to sign is the SNIP-8
//! Poseidon transaction-hash felt, and the broadcast form is JSON — the
//! `BROADCASTED_INVOKE_TXN_V3` body of the `starknet_addInvokeTransaction` RPC method.
//!
//! ###### Example:
//!
//! ```rust
//! use omni_transaction::starknet::types::{Call, DataAvailabilityMode, Felt, ResourceBounds};
//! use omni_transaction::starknet::utils::{encode_calls, CHAIN_ID_SEPOLIA};
//! use omni_transaction::starknet::StarknetTransaction;
//!
//! let calls = vec![Call {
//!     to: Felt::from_hex("0x4138fd51f90d171df37e9d4419c8cdb67d525840c58f8a5c347be93a1c5277d")
//!         .unwrap(),
//!     selector: Felt::from_hex(
//!         "0x2468d193cd15b621b24c2a602b8dbcfa5eaa14f88416c40c09d7fd12592cb4b",
//!     )
//!     .unwrap(),
//!     calldata: vec![],
//! }];
//!
//! let tx = StarknetTransaction {
//!     chain_id: CHAIN_ID_SEPOLIA,
//!     sender_address: Felt::from_hex(
//!         "0x745d525a3582e91299d8d7c71730ffc4b1f191f5b219d800334bc0edad0983b",
//!     )
//!     .unwrap(),
//!     nonce: Felt::from_hex("0x9803").unwrap(),
//!     calldata: encode_calls(&calls),
//!     tip: 0,
//!     l1_gas: ResourceBounds::new(0x186a0, 0x2d79883d20000),
//!     l2_gas: ResourceBounds::new(0x5f5e100, 0xba43b7400),
//!     l1_data_gas: ResourceBounds::new(0x186a0, 0x2d79883d20000),
//!     paymaster_data: vec![],
//!     account_deployment_data: vec![],
//!     nonce_data_availability_mode: DataAvailabilityMode::L1,
//!     fee_data_availability_mode: DataAvailabilityMode::L1,
//! };
//!
//! // The payload to sign is the 32-byte big-endian transaction-hash felt.
//! let payload = tx.build_for_signing();
//! assert_eq!(
//!     payload,
//!     hex::decode("076b52e17bc09064bd986ead34263e6305ef3cecfb3ae9e19b86bf4f1a1a20ea").unwrap()
//! );
//!
//! # #[cfg(feature = "serde_json")]
//! # {
//! // Once signed, the broadcastable JSON body is assembled from the signature felts.
//! let signature = vec![
//!     Felt::from_hex("0x17bacc700df6c82682139e8e550078a5daa75dfe356577f78f7e57fd7c56245")
//!         .unwrap(),
//!     Felt::from_hex("0x4eb8734727eb9412b79ba6d14ff1c9a6beb0dc0b811e3f97168c747f8d427b3")
//!         .unwrap(),
//! ];
//! let json_body = tx.build_with_signature(&signature);
//! assert!(!json_body.is_empty());
//! # }
//! ```
use starknet_crypto::poseidon_hash_many;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use super::types::{DataAvailabilityMode, Felt, ResourceBounds};

/// The Cairo short string `"invoke"`, the SNIP-8 transaction-hash prefix for INVOKE.
const PREFIX_INVOKE: Felt = Felt::from_hex_unchecked("0x696e766f6b65");

/// The version felt of an INVOKE v3 transaction meant for broadcasting.
///
/// (The query-only variant `2^128 + 3` used for fee estimation is deliberately not
/// supported: [`StarknetTransaction::build_for_signing`] always targets real broadcasts.)
const VERSION: Felt = Felt::THREE;

/// Number of bits the nonce DA mode is shifted by when packing both
/// data-availability modes into a single felt.
const DATA_AVAILABILITY_MODE_BITS: u32 = 32;

/// Resource name used in the fee-fields hash for the `l1_gas` bound.
const L1_GAS_NAME: &[u8] = b"L1_GAS";
/// Resource name used in the fee-fields hash for the `l2_gas` bound.
const L2_GAS_NAME: &[u8] = b"L2_GAS";
/// Resource name used in the fee-fields hash for the `l1_data_gas` bound.
/// Note: the hash label is `"L1_DATA"` even though the JSON field is `l1_data_gas`.
const L1_DATA_NAME: &[u8] = b"L1_DATA";

/// A Starknet INVOKE v3 transaction.
///
/// [`Self::build_for_signing`] returns the 32-byte big-endian SNIP-8 transaction-hash felt
/// (the exact value account contracts verify signatures against — nothing hashes it again),
/// and `build_with_signature` (requires the `serde_json` feature) returns the
/// `BROADCASTED_INVOKE_TXN_V3` JSON body for `starknet_addInvokeTransaction`.
///
/// Note: no `schemars::JsonSchema` derive because [`Felt`] (from `starknet-types-core`)
/// does not implement `JsonSchema`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct StarknetTransaction {
    /// Chain id felt: the Cairo short string of the network name, e.g.
    /// [`CHAIN_ID_MAINNET`](crate::starknet::utils::CHAIN_ID_MAINNET) (`"SN_MAIN"`) or
    /// [`CHAIN_ID_SEPOLIA`](crate::starknet::utils::CHAIN_ID_SEPOLIA) (`"SN_SEPOLIA"`).
    /// Custom appchains use their own short string
    /// (see [`cairo_short_string_to_felt`](crate::starknet::utils::cairo_short_string_to_felt)).
    pub chain_id: Felt,
    /// Address of the sender account contract.
    pub sender_address: Felt,
    /// Account nonce. A felt on the wire (not a `u64`).
    pub nonce: Felt,
    /// The already-encoded `__execute__` calldata felts
    /// (see [`encode_calls`](crate::starknet::utils::encode_calls)).
    #[cfg_attr(feature = "serde", serde(default))]
    pub calldata: Vec<Felt>,
    /// Tip to the sequencer, in fri. A `u64` on the wire.
    #[cfg_attr(feature = "serde", serde(default))]
    pub tip: u64,
    /// Resource bounds for L1 gas.
    #[cfg_attr(feature = "serde", serde(default))]
    pub l1_gas: ResourceBounds,
    /// Resource bounds for L2 gas.
    #[cfg_attr(feature = "serde", serde(default))]
    pub l2_gas: ResourceBounds,
    /// Resource bounds for L1 data gas (blob data). Required since Starknet 0.13.4.
    #[cfg_attr(feature = "serde", serde(default))]
    pub l1_data_gas: ResourceBounds,
    /// Paymaster data. Must be empty on current networks.
    #[cfg_attr(feature = "serde", serde(default))]
    pub paymaster_data: Vec<Felt>,
    /// Account deployment data. Must be empty on current networks.
    #[cfg_attr(feature = "serde", serde(default))]
    pub account_deployment_data: Vec<Felt>,
    /// Data-availability mode of the nonce. `L1` on current networks.
    #[cfg_attr(feature = "serde", serde(default))]
    pub nonce_data_availability_mode: DataAvailabilityMode,
    /// Data-availability mode of the fee. `L1` on current networks.
    #[cfg_attr(feature = "serde", serde(default))]
    pub fee_data_availability_mode: DataAvailabilityMode,
}

impl StarknetTransaction {
    /// Computes the SNIP-8 v3 transaction-hash felt:
    ///
    /// ```text
    /// poseidon_hash_many([
    ///     "invoke", 3, sender_address, fee_fields_hash, h(paymaster_data),
    ///     chain_id, nonce, data_availability_modes, h(account_deployment_data), h(calldata),
    /// ])
    /// ```
    pub fn compute_hash(&self) -> Felt {
        let fields = [
            PREFIX_INVOKE,
            VERSION,
            self.sender_address,
            self.fee_fields_hash(),
            poseidon_hash_many(&self.paymaster_data),
            self.chain_id,
            self.nonce,
            self.data_availability_modes_felt(),
            poseidon_hash_many(&self.account_deployment_data),
            poseidon_hash_many(&self.calldata),
        ];

        poseidon_hash_many(&fields)
    }

    /// Returns the payload to sign: the 32-byte big-endian encoding of the
    /// transaction-hash felt returned by [`Self::compute_hash`].
    ///
    /// This is a final digest, not a preimage — account contracts receive exactly this
    /// felt in `__validate__` and verify the signature against it, so no further hashing
    /// may be applied by either the signer or the account. Because a felt is smaller
    /// than `2^252`, the first byte is always `<= 0x08`.
    pub fn build_for_signing(&self) -> Vec<u8> {
        self.compute_hash().to_bytes_be().to_vec()
    }

    /// Returns the signed transaction as JSON bytes of the `BROADCASTED_INVOKE_TXN_V3`
    /// object, i.e. the `invoke_transaction` param value of `starknet_addInvokeTransaction`.
    ///
    /// The signature is an opaque list of felts whose layout is defined by the sender's
    /// account contract (see [`Signature`](crate::starknet::types::Signature)); the
    /// signature is *not* part of the transaction hash, so [`Self::compute_hash`] is
    /// unchanged by it.
    ///
    /// Without the `serde_json` feature this method is unavailable; the raw hash
    /// ([`Self::compute_hash`], [`Self::build_for_signing`]) and all public fields remain
    /// accessible so callers can assemble the JSON themselves.
    #[cfg(feature = "serde_json")]
    pub fn build_with_signature(&self, signature: &[Felt]) -> Vec<u8> {
        serde_json::to_vec(&self.to_broadcasted_json_value(signature))
            .expect("BROADCASTED_INVOKE_TXN_V3 serialization is infallible")
    }

    /// Returns the full JSON-RPC `params` object for `starknet_addInvokeTransaction`,
    /// i.e. `{"invoke_transaction": <BROADCASTED_INVOKE_TXN_V3>}`, as JSON bytes.
    #[cfg(feature = "serde_json")]
    pub fn to_rpc_params_json(&self, signature: &[Felt]) -> Vec<u8> {
        serde_json::to_vec(&serde_json::json!({
            "invoke_transaction": self.to_broadcasted_json_value(signature),
        }))
        .expect("RPC params serialization is infallible")
    }

    /// Constructs a [`StarknetTransaction`] from its JSON representation
    /// (the serde format of this struct; missing optional fields default).
    #[cfg(feature = "serde_json")]
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// The `BROADCASTED_INVOKE_TXN_V3` object as a `serde_json::Value`
    /// (all numeric strings in minimal lowercase hex, per the RPC spec).
    #[cfg(feature = "serde_json")]
    fn to_broadcasted_json_value(&self, signature: &[Felt]) -> serde_json::Value {
        serde_json::json!({
            "type": "INVOKE",
            "version": "0x3",
            "sender_address": self.sender_address,
            "calldata": self.calldata,
            "signature": signature,
            "nonce": self.nonce,
            "resource_bounds": {
                "l1_gas": resource_bounds_json(&self.l1_gas),
                "l1_data_gas": resource_bounds_json(&self.l1_data_gas),
                "l2_gas": resource_bounds_json(&self.l2_gas),
            },
            "tip": format!("{:#x}", self.tip),
            "paymaster_data": self.paymaster_data,
            "account_deployment_data": self.account_deployment_data,
            "nonce_data_availability_mode": self.nonce_data_availability_mode.as_str(),
            "fee_data_availability_mode": self.fee_data_availability_mode.as_str(),
        })
    }

    /// The fee-fields hash: `poseidon_hash_many([tip, L1_GAS, L2_GAS, L1_DATA])`,
    /// each resource packed as `name << 192 | max_amount << 128 | max_price_per_unit`.
    fn fee_fields_hash(&self) -> Felt {
        let fee_fields = [
            Felt::from(self.tip),
            resource_bounds_felt(L1_GAS_NAME, &self.l1_gas),
            resource_bounds_felt(L2_GAS_NAME, &self.l2_gas),
            resource_bounds_felt(L1_DATA_NAME, &self.l1_data_gas),
        ];

        poseidon_hash_many(&fee_fields)
    }

    /// Both data-availability modes packed into one felt:
    /// `(nonce_da_mode << 32) + fee_da_mode`.
    fn data_availability_modes_felt(&self) -> Felt {
        Felt::from(
            (self.nonce_data_availability_mode.value() << DATA_AVAILABILITY_MODE_BITS)
                + self.fee_data_availability_mode.value(),
        )
    }
}

/// Packs one resource bound into a felt, big-endian:
/// byte 0 zero, bytes 1..8 the resource name right-aligned,
/// bytes 8..16 `max_amount` as u64, bytes 16..32 `max_price_per_unit` as u128.
fn resource_bounds_felt(name: &[u8], bounds: &ResourceBounds) -> Felt {
    debug_assert!(name.len() <= 7, "resource name must fit in 7 bytes");

    let mut buffer = [0u8; 32];
    buffer[8 - name.len()..8].copy_from_slice(name);
    buffer[8..16].copy_from_slice(&bounds.max_amount.to_be_bytes());
    buffer[16..32].copy_from_slice(&bounds.max_price_per_unit.to_be_bytes());

    Felt::from_bytes_be(&buffer)
}

/// One `RESOURCE_BOUNDS` JSON object with `u64`/`u128` values as minimal-hex strings.
#[cfg(feature = "serde_json")]
fn resource_bounds_json(bounds: &ResourceBounds) -> serde_json::Value {
    serde_json::json!({
        "max_amount": format!("{:#x}", bounds.max_amount),
        "max_price_per_unit": format!("{:#x}", bounds.max_price_per_unit),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::starknet::types::Call;
    use crate::starknet::utils::{encode_calls, CHAIN_ID_MAINNET, CHAIN_ID_SEPOLIA};

    /// Real Sepolia invoke v3 with three resource bounds (block 567941, ACCEPTED_ON_L1):
    /// 0x76b52e17bc09064bd986ead34263e6305ef3cecfb3ae9e19b86bf4f1a1a20ea.
    /// Fixture from NethermindEth/juno feeder testdata.
    fn sepolia_reference_transaction() -> StarknetTransaction {
        StarknetTransaction {
            chain_id: CHAIN_ID_SEPOLIA,
            sender_address: Felt::from_hex_unchecked(
                "0x745d525a3582e91299d8d7c71730ffc4b1f191f5b219d800334bc0edad0983b",
            ),
            nonce: Felt::from_hex_unchecked("0x9803"),
            calldata: vec![
                Felt::from_hex_unchecked("0x1"),
                Felt::from_hex_unchecked(
                    "0x4138fd51f90d171df37e9d4419c8cdb67d525840c58f8a5c347be93a1c5277d",
                ),
                Felt::from_hex_unchecked(
                    "0x2468d193cd15b621b24c2a602b8dbcfa5eaa14f88416c40c09d7fd12592cb4b",
                ),
                Felt::from_hex_unchecked("0x0"),
            ],
            tip: 0,
            l1_gas: ResourceBounds::new(0x186a0, 0x2d79883d20000),
            l2_gas: ResourceBounds::new(0x5f5e100, 0xba43b7400),
            l1_data_gas: ResourceBounds::new(0x186a0, 0x2d79883d20000),
            paymaster_data: vec![],
            account_deployment_data: vec![],
            nonce_data_availability_mode: DataAvailabilityMode::L1,
            fee_data_availability_mode: DataAvailabilityMode::L1,
        }
    }

    const SEPOLIA_EXPECTED_HASH: &str =
        "0x76b52e17bc09064bd986ead34263e6305ef3cecfb3ae9e19b86bf4f1a1a20ea";

    #[test]
    fn test_compute_hash_matches_real_sepolia_transaction() {
        let tx = sepolia_reference_transaction();
        assert_eq!(
            tx.compute_hash(),
            Felt::from_hex_unchecked(SEPOLIA_EXPECTED_HASH)
        );
    }

    #[test]
    fn test_build_for_signing_is_32_byte_big_endian_hash() {
        let tx = sepolia_reference_transaction();
        let payload = tx.build_for_signing();

        assert_eq!(payload.len(), 32);
        assert_eq!(
            hex::encode(&payload),
            "076b52e17bc09064bd986ead34263e6305ef3cecfb3ae9e19b86bf4f1a1a20ea"
        );
    }

    #[test]
    fn test_fee_fields_intermediates_match_reference() {
        let tx = sepolia_reference_transaction();

        assert_eq!(
            resource_bounds_felt(L1_GAS_NAME, &tx.l1_gas),
            Felt::from_hex_unchecked(
                "0x4c315f47415300000000000186a000000000000000000002d79883d20000"
            )
        );
        assert_eq!(
            resource_bounds_felt(L2_GAS_NAME, &tx.l2_gas),
            Felt::from_hex_unchecked(
                "0x4c325f4741530000000005f5e10000000000000000000000000ba43b7400"
            )
        );
        assert_eq!(
            resource_bounds_felt(L1_DATA_NAME, &tx.l1_data_gas),
            Felt::from_hex_unchecked(
                "0x4c315f4441544100000000000186a000000000000000000002d79883d20000"
            )
        );
        assert_eq!(
            tx.fee_fields_hash(),
            Felt::from_hex_unchecked(
                "0x1521540779c76d426b6f6f3d89be43ab2a037dcff677f5083d776cb6f31ce3a"
            )
        );
        assert_eq!(tx.data_availability_modes_felt(), Felt::ZERO);
    }

    #[test]
    fn test_poseidon_hash_of_empty_list() {
        let empty: [Felt; 0] = [];
        assert_eq!(
            poseidon_hash_many(&empty),
            Felt::from_hex_unchecked(
                "0x2272be0f580fd156823304800919530eaa97430e972d7213ee13f4fbf7a5dbc"
            )
        );
    }

    /// Historical mainnet invoke v3 hashed with only two resource bounds (pre-0.13.4):
    /// 0x1d4735f4ba73a67be2f648d9b21cab3783383b8c229566b46b027c46012219 (block 636864),
    /// from StarkWare's sequencer transaction_hash.json fixture.
    ///
    /// The library only emits three-resource transactions (current network requirement),
    /// so this vector is reassembled manually to pin down the resource-felt packing and
    /// the overall field ordering.
    #[test]
    fn test_historical_mainnet_two_resource_vector() {
        let sender = Felt::from_hex_unchecked(
            "0x69c0f9bcd79697bdceaf7748e3ff8f34aa39e4063ce44896af664c0c96f6c10",
        );
        let nonce = Felt::from_hex_unchecked("0x9d");
        let calldata = [
            Felt::from_hex_unchecked("0x1"),
            Felt::from_hex_unchecked(
                "0x4c0a5193d58f74fbace4b74dcf65481e734ed1714121bdc571da345540efa05",
            ),
            Felt::from_hex_unchecked(
                "0x3943907ef0ef6f9d2e2408b05e520a66daaf74293dbf665e5a20b117676170e",
            ),
            Felt::from_hex_unchecked("0x2"),
            Felt::from_hex_unchecked(
                "0x49d36570d4e46f48e99674bd3fcc84644ddd6b96f7c741b1562b82f9e004dc7",
            ),
            Felt::from_hex_unchecked("0x16345785d8a0000"),
        ];

        // Pre-0.13.4 fee hash: tip, L1_GAS, L2_GAS — no L1_DATA felt.
        let fee_fields_hash = poseidon_hash_many(&[
            Felt::ZERO,
            resource_bounds_felt(L1_GAS_NAME, &ResourceBounds::new(0xa9e, 0x7f2a1ad4f2f1)),
            resource_bounds_felt(L2_GAS_NAME, &ResourceBounds::new(0, 0)),
        ]);

        let empty: [Felt; 0] = [];
        let hash = poseidon_hash_many(&[
            PREFIX_INVOKE,
            VERSION,
            sender,
            fee_fields_hash,
            poseidon_hash_many(&empty),
            CHAIN_ID_MAINNET,
            nonce,
            Felt::ZERO,
            poseidon_hash_many(&empty),
            poseidon_hash_many(&calldata),
        ]);

        assert_eq!(
            hash,
            Felt::from_hex_unchecked(
                "0x1d4735f4ba73a67be2f648d9b21cab3783383b8c229566b46b027c46012219"
            )
        );
    }

    #[test]
    fn test_calldata_from_encode_calls_matches_on_chain_calldata() {
        let calls = [Call {
            to: Felt::from_hex_unchecked(
                "0x4138fd51f90d171df37e9d4419c8cdb67d525840c58f8a5c347be93a1c5277d",
            ),
            selector: Felt::from_hex_unchecked(
                "0x2468d193cd15b621b24c2a602b8dbcfa5eaa14f88416c40c09d7fd12592cb4b",
            ),
            calldata: vec![],
        }];

        assert_eq!(
            encode_calls(&calls),
            sepolia_reference_transaction().calldata
        );
    }

    #[test]
    fn test_data_availability_modes_packing() {
        let mut tx = sepolia_reference_transaction();

        tx.nonce_data_availability_mode = DataAvailabilityMode::L2;
        tx.fee_data_availability_mode = DataAvailabilityMode::L1;
        assert_eq!(
            tx.data_availability_modes_felt(),
            Felt::from(1u64 << 32),
            "nonce DA mode occupies the high 32 bits"
        );

        tx.nonce_data_availability_mode = DataAvailabilityMode::L1;
        tx.fee_data_availability_mode = DataAvailabilityMode::L2;
        assert_eq!(tx.data_availability_modes_felt(), Felt::from(1u64));
    }

    #[cfg(feature = "serde_json")]
    #[test]
    fn test_build_with_signature_matches_broadcasted_invoke_txn_v3() {
        let tx = sepolia_reference_transaction();

        // On-chain signature of the real sepolia transaction (Stark-curve [r, s]).
        let signature = vec![
            Felt::from_hex_unchecked(
                "0x17bacc700df6c82682139e8e550078a5daa75dfe356577f78f7e57fd7c56245",
            ),
            Felt::from_hex_unchecked(
                "0x4eb8734727eb9412b79ba6d14ff1c9a6beb0dc0b811e3f97168c747f8d427b3",
            ),
        ];

        let encoded = tx.build_with_signature(&signature);
        let actual: serde_json::Value = serde_json::from_slice(&encoded).unwrap();

        // Literal BROADCASTED_INVOKE_TXN_V3 per starknet-specs v0.9.0.
        let expected: serde_json::Value = serde_json::json!({
            "type": "INVOKE",
            "version": "0x3",
            "sender_address": "0x745d525a3582e91299d8d7c71730ffc4b1f191f5b219d800334bc0edad0983b",
            "calldata": [
                "0x1",
                "0x4138fd51f90d171df37e9d4419c8cdb67d525840c58f8a5c347be93a1c5277d",
                "0x2468d193cd15b621b24c2a602b8dbcfa5eaa14f88416c40c09d7fd12592cb4b",
                "0x0"
            ],
            "signature": [
                "0x17bacc700df6c82682139e8e550078a5daa75dfe356577f78f7e57fd7c56245",
                "0x4eb8734727eb9412b79ba6d14ff1c9a6beb0dc0b811e3f97168c747f8d427b3"
            ],
            "nonce": "0x9803",
            "resource_bounds": {
                "l1_gas": {
                    "max_amount": "0x186a0",
                    "max_price_per_unit": "0x2d79883d20000"
                },
                "l1_data_gas": {
                    "max_amount": "0x186a0",
                    "max_price_per_unit": "0x2d79883d20000"
                },
                "l2_gas": {
                    "max_amount": "0x5f5e100",
                    "max_price_per_unit": "0xba43b7400"
                }
            },
            "tip": "0x0",
            "paymaster_data": [],
            "account_deployment_data": [],
            "nonce_data_availability_mode": "L1",
            "fee_data_availability_mode": "L1"
        });

        assert_eq!(actual, expected);
    }

    #[cfg(feature = "serde_json")]
    #[test]
    fn test_broadcast_json_round_trips_to_same_hash() {
        let tx = sepolia_reference_transaction();
        let encoded = tx.build_with_signature(&[]);
        let value: serde_json::Value = serde_json::from_slice(&encoded).unwrap();

        // Rebuild the transaction from the broadcast JSON (plus the chain id, which is
        // implied by the RPC endpoint, not part of the broadcast body).
        let parse_felt = |v: &serde_json::Value| Felt::from_hex(v.as_str().unwrap()).unwrap();
        let parse_felts = |v: &serde_json::Value| -> Vec<Felt> {
            v.as_array().unwrap().iter().map(parse_felt).collect()
        };
        let parse_u128 = |v: &serde_json::Value| {
            u128::from_str_radix(v.as_str().unwrap().strip_prefix("0x").unwrap(), 16).unwrap()
        };
        let parse_bounds = |v: &serde_json::Value| {
            ResourceBounds::new(
                parse_u128(&v["max_amount"]) as u64,
                parse_u128(&v["max_price_per_unit"]),
            )
        };

        let rebuilt = StarknetTransaction {
            chain_id: CHAIN_ID_SEPOLIA,
            sender_address: parse_felt(&value["sender_address"]),
            nonce: parse_felt(&value["nonce"]),
            calldata: parse_felts(&value["calldata"]),
            tip: parse_u128(&value["tip"]) as u64,
            l1_gas: parse_bounds(&value["resource_bounds"]["l1_gas"]),
            l2_gas: parse_bounds(&value["resource_bounds"]["l2_gas"]),
            l1_data_gas: parse_bounds(&value["resource_bounds"]["l1_data_gas"]),
            paymaster_data: parse_felts(&value["paymaster_data"]),
            account_deployment_data: parse_felts(&value["account_deployment_data"]),
            nonce_data_availability_mode: DataAvailabilityMode::L1,
            fee_data_availability_mode: DataAvailabilityMode::L1,
        };

        assert_eq!(
            rebuilt.compute_hash(),
            Felt::from_hex_unchecked(SEPOLIA_EXPECTED_HASH),
            "broadcast JSON and signing hash must describe the same transaction"
        );
    }

    #[cfg(feature = "serde_json")]
    #[test]
    fn test_to_rpc_params_json_wraps_invoke_transaction() {
        let tx = sepolia_reference_transaction();
        let params = tx.to_rpc_params_json(&[]);
        let value: serde_json::Value = serde_json::from_slice(&params).unwrap();

        let body: serde_json::Value =
            serde_json::from_slice(&tx.build_with_signature(&[])).unwrap();
        assert_eq!(value["invoke_transaction"], body);
    }

    #[cfg(feature = "serde_json")]
    #[test]
    fn test_from_json() {
        let json = r#"
        {
            "chain_id": "0x534e5f5345504f4c4941",
            "sender_address": "0x745d525a3582e91299d8d7c71730ffc4b1f191f5b219d800334bc0edad0983b",
            "nonce": "0x9803",
            "calldata": [
                "0x1",
                "0x4138fd51f90d171df37e9d4419c8cdb67d525840c58f8a5c347be93a1c5277d",
                "0x2468d193cd15b621b24c2a602b8dbcfa5eaa14f88416c40c09d7fd12592cb4b",
                "0x0"
            ],
            "l1_gas": { "max_amount": 100000, "max_price_per_unit": 800000000000000 },
            "l2_gas": { "max_amount": 100000000, "max_price_per_unit": 50000000000 },
            "l1_data_gas": { "max_amount": 100000, "max_price_per_unit": 800000000000000 }
        }"#;

        let tx = StarknetTransaction::from_json(json).unwrap();

        assert_eq!(tx, sepolia_reference_transaction());
        assert_eq!(
            tx.compute_hash(),
            Felt::from_hex_unchecked(SEPOLIA_EXPECTED_HASH)
        );
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serde_round_trip() {
        let tx = sepolia_reference_transaction();
        let json = serde_json::to_string(&tx).unwrap();
        let decoded: StarknetTransaction = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, tx);
    }
}
