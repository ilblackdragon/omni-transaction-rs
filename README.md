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
    .signer_public_key(signer_public_key.to_public_key().unwrap())
    .nonce(nonce)
    .receiver_id(receiver_id.to_string())
    .block_hash(block_hash_str.to_block_hash().unwrap())
    .actions(actions)
    .build();

// Now you have access to build_for_signing that returns the encoded payload
let near_tx_encoded = near_tx.build_for_signing();
```

Building an Ethereum transaction:

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

Building a Bitcoin transaction:

```rust
let txid_str = "2ece6cd71fee90ff613cee8f30a52c3ecc58685acf9b817b9c467b7ff199871c";
let hash = Hash::from_hex(txid_str).unwrap();
let txid = Txid(hash);
let vout = 0;

let txin: TxIn = TxIn {
    previous_output: OutPoint::new(txid, vout as u32),
    script_sig: ScriptBuf::default(), // For a p2pkh script_sig is initially empty.
    sequence: Sequence::MAX,
    witness: Witness::default(),
};

let sender_script_pubkey_hex = "76a914cb8a3018cf279311b148cb8d13728bd8cbe95bda88ac";
let sender_script_pubkey = ScriptBuf(sender_script_pubkey_hex.as_bytes().to_vec());

let receiver_script_pubkey_hex = "76a914406cf8a18b97a230d15ed82f0d251560a05bda0688ac";
let receiver_script_pubkey = ScriptBuf(receiver_script_pubkey_hex.as_bytes().to_vec());

// The spend output is locked to a key controlled by the receiver.
let spend_txout: TxOut = TxOut {
    value: Amount::from_sat(500_000_000),
    script_pubkey: receiver_script_pubkey,
};

let change_txout = TxOut {
    value: Amount::from_sat(100_000_000),
    script_pubkey: sender_script_pubkey,
};

let bitcoin_tx = TransactionBuilder::new::<BITCOIN>()
    .version(Version::One)
    .inputs(vec![txin])
    .outputs(vec![spend_txout, change_txout])
    .lock_time(LockTime::from_height(0).unwrap())
    .build();

// Prepare the transaction for signing
let encoded_tx = bitcoin_tx.build_for_signing_legacy(EcdsaSighashType::All);
```