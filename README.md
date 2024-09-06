# Omni Transaction Rust library

Library to construct transactions for different chains inside Near contracts and Rust clients.

[![Telegram chat][telegram-badge]][telegram-url]

[telegram-badge]: https://img.shields.io/endpoint?color=neon&style=for-the-badge&url=https://tg.sumanjay.workers.dev/chain_abstraction
[telegram-url]: https://t.me/chain_abstraction

## Supported chains

- NEAR
- Ethereum
- Bitcoin (Coming soon)

## Examples

For a complete set of examples see the [examples](https://github.com/Omni-rs/examples.git) repository.

Building a NEAR transaction:
```rust
let signer_id = "alice.near";
let signer_public_key = "ed25519:6E8sCci9badyRkXb3JoRpBj5p8C6Tw41ELDZoiihKEtp";
let nonce = U64(0);
let receiver_id = "bob.near";
let block_hash_str = "4reLvkAWfqk5fsqio1KLudk46cqRz9erQdaHkWZKMJDZ";
let transfer_action = Action::Transfer(TransferAction { deposit: U128(1) });
let actions = vec![transfer_action];

let near_tx = TransactionBuilder::new::<NEAR>()
        .signer_id(signer_id.to_string())
        .signer_public_key(alice_public_key.to_public_key().unwrap())
        .nonce(nonce)
        .receiver_id(receiver_id.to_string())
        .block_hash(block_hash_str.to_block_hash().unwrap())
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
