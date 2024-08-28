# Omni Transaction Rust library

Library to be used inside Rust smart contracts to construct Transactions for different chains.

## Examples

Building a NEAR transaction:
```rust
let signer_id = "alice.near";
let signer_public_key = [0u8; 64];
let nonce = 0;
let receiver_id = "bob.near";
let block_hash = [0u8; 32];
let transfer_action = Action::Transfer(TransferAction { deposit: 1u128 });
let actions = vec![transfer_action];

let near_tx = TransactionBuilder::new::<NEAR>()
        .signer_id(signer_id.to_string())
        .signer_public_key(PublicKey::SECP256K1(signer_public_key.into()))
        .nonce(nonce)
        .receiver_id(receiver_id.to_string())
        .block_hash(block_hash)
        .actions(actions)
        .build();

// Now you have access to build_for_signing that returns the encoded payload
let near_tx_encoded = near_tx.build_for_signing();
```

Building Ethereum transaction:
```rust
let to_address_str = "d8dA6BF26964aF9D7eEd9e03E53415D37aA96045";
let to_address = parse_eth_address(to_address_str);
let max_gas_fee: u128 = 20_000_000_000;
let max_priority_fee_per_gas: u128 = 1_000_000_000;
let gas_limit: u128 = 21_000;
let chain_id: u64 = 1;
let nonce: u64 = 0;
let data: Vec<u8> = vec![];
let value: u128 = 10000000000000000; // 0.01 ETH

let evm_tx = TransactionBuilder::new::<EVM>()
        .nonce(nonce)
        .to(to_address)
        .value(value)
        .input(data.clone())
        .max_priority_fee_per_gas(max_priority_fee_per_gas)
        .max_fee_per_gas(max_gas_fee)
        .gas_limit(gas_limit)
        .chain_id(chain_id)
        .build();

// Now you have access to build_for_signing that returns the encoded payload
let rlp_encoded = evm_tx.build_for_signing();
```
