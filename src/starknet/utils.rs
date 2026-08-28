//! Utility functions and constants for Starknet transactions.
use sha3::{Digest, Keccak256};

use super::types::{Call, Felt};

/// Chain id of Starknet mainnet: the Cairo short string `"SN_MAIN"`.
pub const CHAIN_ID_MAINNET: Felt = Felt::from_hex_unchecked("0x534e5f4d41494e");

/// Chain id of the Starknet Sepolia testnet: the Cairo short string `"SN_SEPOLIA"`.
pub const CHAIN_ID_SEPOLIA: Felt = Felt::from_hex_unchecked("0x534e5f5345504f4c4941");

/// Entry-point selector of the SNIP-6 `__execute__` function,
/// i.e. `starknet_keccak("__execute__")`.
pub const SELECTOR_EXECUTE: Felt =
    Felt::from_hex_unchecked("0x15d40a3d6ca2ac30f4031e42be28da9b056fef9bb7357ac5e85627ee876e5ad");

/// Computes the Starknet variant of Keccak: `keccak256(data)` with the top 6 bits
/// masked off, so the result fits in 250 bits (`keccak256(data) mod 2^250`).
pub fn starknet_keccak(data: &[u8]) -> Felt {
    let mut hash: [u8; 32] = Keccak256::digest(data).into();
    hash[0] &= 0x03; // mask the 6 most significant bits => value < 2^250
    Felt::from_bytes_be(&hash)
}

/// Computes the entry-point selector for a function name:
/// `starknet_keccak` of the name's bytes, with the special cases `"__default__"`
/// and `"__l1_default__"` mapping to `0`.
pub fn get_selector_from_name(name: &str) -> Felt {
    if name == "__default__" || name == "__l1_default__" {
        Felt::ZERO
    } else {
        starknet_keccak(name.as_bytes())
    }
}

/// Converts a Cairo short string (at most 31 ASCII characters) to its felt encoding:
/// the raw ASCII bytes interpreted as a big-endian integer.
///
/// Useful to build the chain id of custom Starknet appchains
/// (e.g. `cairo_short_string_to_felt("SN_MAIN") == CHAIN_ID_MAINNET`).
///
/// # Panics
///
/// Panics if the string is longer than 31 characters or contains non-ASCII characters.
pub fn cairo_short_string_to_felt(s: &str) -> Felt {
    assert!(s.is_ascii(), "short string must be ASCII");
    assert!(s.len() <= 31, "short string must be at most 31 characters");
    Felt::from_bytes_be_slice(s.as_bytes())
}

/// Encodes a list of [`Call`]s into `__execute__` calldata felts using the SNIP-6
/// "new" (Cairo 1) encoding, i.e. the Cairo Serde of `Array<Call>`:
///
/// `[n_calls, (to, selector, calldata_len, calldata...) for each call]`
pub fn encode_calls(calls: &[Call]) -> Vec<Felt> {
    let mut execute_calldata =
        Vec::with_capacity(1 + calls.iter().map(|c| 3 + c.calldata.len()).sum::<usize>());

    execute_calldata.push(Felt::from(calls.len() as u64));

    for call in calls {
        execute_calldata.push(call.to);
        execute_calldata.push(call.selector);
        execute_calldata.push(Felt::from(call.calldata.len() as u64));
        execute_calldata.extend_from_slice(&call.calldata);
    }

    execute_calldata
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chain_id_constants_match_short_strings() {
        assert_eq!(cairo_short_string_to_felt("SN_MAIN"), CHAIN_ID_MAINNET);
        assert_eq!(cairo_short_string_to_felt("SN_SEPOLIA"), CHAIN_ID_SEPOLIA);
        assert_eq!(
            cairo_short_string_to_felt("invoke"),
            Felt::from_hex_unchecked("0x696e766f6b65")
        );
    }

    #[test]
    fn test_chain_id_constants_match_starknet_core() {
        assert_eq!(CHAIN_ID_MAINNET, starknet_core::chain_id::MAINNET);
        assert_eq!(CHAIN_ID_SEPOLIA, starknet_core::chain_id::SEPOLIA);
    }

    #[test]
    fn test_cairo_short_string_to_felt_matches_starknet_core() {
        for s in ["invoke", "SN_MAIN", "SN_SEPOLIA", "a", "", "L1_DATA"] {
            assert_eq!(
                cairo_short_string_to_felt(s),
                starknet_core::utils::cairo_short_string_to_felt(s).unwrap(),
                "short string mismatch for {s:?}"
            );
        }
    }

    #[test]
    fn test_selector_vectors() {
        assert_eq!(get_selector_from_name("__execute__"), SELECTOR_EXECUTE);
        assert_eq!(
            get_selector_from_name("transfer"),
            Felt::from_hex_unchecked(
                "0x83afd3f4caedc6eebf44246fe54e38c95e3179a5ec9ea81740eca5b482d12e"
            )
        );
        assert_eq!(get_selector_from_name("__default__"), Felt::ZERO);
        assert_eq!(get_selector_from_name("__l1_default__"), Felt::ZERO);
    }

    #[test]
    fn test_selectors_match_starknet_core() {
        for name in [
            "__execute__",
            "__validate__",
            "transfer",
            "approve",
            "balanceOf",
            "__default__",
            "__l1_default__",
        ] {
            assert_eq!(
                get_selector_from_name(name),
                starknet_core::utils::get_selector_from_name(name).unwrap(),
                "selector mismatch for {name:?}"
            );
        }
    }

    #[test]
    fn test_encode_calls_single_call_with_args() {
        // Matches the on-chain calldata of mainnet tx
        // 0x1d4735f4ba73a67be2f648d9b21cab3783383b8c229566b46b027c46012219 (block 636864).
        let calls = [Call {
            to: Felt::from_hex_unchecked(
                "0x4c0a5193d58f74fbace4b74dcf65481e734ed1714121bdc571da345540efa05",
            ),
            selector: Felt::from_hex_unchecked(
                "0x3943907ef0ef6f9d2e2408b05e520a66daaf74293dbf665e5a20b117676170e",
            ),
            calldata: vec![
                Felt::from_hex_unchecked(
                    "0x49d36570d4e46f48e99674bd3fcc84644ddd6b96f7c741b1562b82f9e004dc7",
                ),
                Felt::from_hex_unchecked("0x16345785d8a0000"),
            ],
        }];

        let expected: Vec<Felt> = [
            "0x1",
            "0x4c0a5193d58f74fbace4b74dcf65481e734ed1714121bdc571da345540efa05",
            "0x3943907ef0ef6f9d2e2408b05e520a66daaf74293dbf665e5a20b117676170e",
            "0x2",
            "0x49d36570d4e46f48e99674bd3fcc84644ddd6b96f7c741b1562b82f9e004dc7",
            "0x16345785d8a0000",
        ]
        .iter()
        .map(|s| Felt::from_hex_unchecked(s))
        .collect();

        assert_eq!(encode_calls(&calls), expected);
    }

    #[test]
    fn test_encode_calls_single_call_without_args() {
        // Matches the on-chain calldata of sepolia tx
        // 0x76b52e17bc09064bd986ead34263e6305ef3cecfb3ae9e19b86bf4f1a1a20ea (block 567941).
        let calls = [Call {
            to: Felt::from_hex_unchecked(
                "0x4138fd51f90d171df37e9d4419c8cdb67d525840c58f8a5c347be93a1c5277d",
            ),
            selector: Felt::from_hex_unchecked(
                "0x2468d193cd15b621b24c2a602b8dbcfa5eaa14f88416c40c09d7fd12592cb4b",
            ),
            calldata: vec![],
        }];

        let expected: Vec<Felt> = [
            "0x1",
            "0x4138fd51f90d171df37e9d4419c8cdb67d525840c58f8a5c347be93a1c5277d",
            "0x2468d193cd15b621b24c2a602b8dbcfa5eaa14f88416c40c09d7fd12592cb4b",
            "0x0",
        ]
        .iter()
        .map(|s| Felt::from_hex_unchecked(s))
        .collect();

        assert_eq!(encode_calls(&calls), expected);
    }

    #[test]
    fn test_encode_calls_empty() {
        assert_eq!(encode_calls(&[]), vec![Felt::ZERO]);
    }
}
