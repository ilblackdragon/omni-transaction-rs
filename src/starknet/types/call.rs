//! A single contract call of a Starknet multicall.
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use starknet_types_core::felt::Felt;

/// A single contract call, as understood by SNIP-6 account contracts
/// (`Call { to: ContractAddress, selector: felt252, calldata: Array<felt252> }`).
///
/// A list of calls is flattened into `__execute__` calldata felts with
/// [`encode_calls`](crate::starknet::utils::encode_calls).
///
/// Note: no `schemars::JsonSchema` derive because [`Felt`] (from `starknet-types-core`)
/// does not implement `JsonSchema`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Call {
    /// Address of the contract being called.
    pub to: Felt,
    /// Entry-point selector, i.e. `starknet_keccak` of the function name
    /// (see [`get_selector_from_name`](crate::starknet::utils::get_selector_from_name)).
    pub selector: Felt,
    /// The raw felt arguments passed to the entry point.
    pub calldata: Vec<Felt>,
}
