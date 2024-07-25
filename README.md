# Omni Transaction Rust library

Library to be used inside Rust smart contracts to construct Transactions for different chains.

## Examples

Building NEAR transaction:
```rust
let bytes = TransactionBuilder::new()
    .sender("alice.near".to_string())
    .signer_public_key([0u8; 64])
    .receiver("bob.near".to_string())
    .amount(100)
    .build(ChainKind::NEAR);
```

Building Ethereum transaction:
```rust
let bytes = TransactionBuilder::new()
    .receiver("0123456789abcdefdeadbeef0123456789abcdef".to_string())
    .amount(100)
    .build(ChainKind::EVM { chain_id: 1 });
```
