# Omni Transaction Rust library

Library to construct transactions for different chains inside Near contracts and Rust clients.

<p>
    <a href="https://docs.rs/omni-transaction"><img src="https://docs.rs/omni-transaction/badge.svg?style=flat-square" alt="Reference Documentation" /></a>
    <a href="https://blog.rust-lang.org/2025/02/20/Rust-1.85.0/"><img src="https://img.shields.io/badge/rustc-1.85+-lightgray.svg?style=flat-square" alt="MSRV" /></a>
    <a href="https://crates.io/crates/omni-transaction"><img src="https://img.shields.io/crates/v/omni-transaction.svg?style=flat-square" alt="Crates.io version" /></a>
    <a href="https://crates.io/crates/omni-transaction"><img src="https://img.shields.io/crates/d/omni-transaction.svg?style=flat-square" alt="Download" /></a>
    <a href="http://near.chat"><img src="https://img.shields.io/discord/490367152054992913?style=flat-square&label=discord&color=lightgreen" alt="Join the community on Discord" /></a>
    <a href="https://t.me/NEAR_Tools_Community_Group"><img src="https://img.shields.io/badge/telegram-online-lightgreen?style=flat-square" alt="Join the community on Telegram" /></a>
    <a href="https://x.com/nearprotocol"><img src="https://img.shields.io/twitter/follow/NEARProtocol" alt="Join the community on Twitter" /></a>
  </p>

[![Telegram chat][telegram-badge]][telegram-url]

[telegram-badge]: https://img.shields.io/endpoint?color=neon&style=for-the-badge&url=https://tg.sumanjay.workers.dev/chain_abstraction
[telegram-url]: https://t.me/chain_abstraction

## Supported chains

- NEAR
- EVM chains (Ethereum, Arbitrum, Base, BNB, Polygon, HyperEVM, Abstract and other L2s — one builder, selected by `chain_id`)
- Bitcoin
- Solana (and other SVM chains such as Fogo)
- Aptos
- Sui
- Zcash (transparent v5 transactions)
- Starknet (invoke v3)
- TON (wallet v5r1 and v4r2)

Each chain lives behind its own feature flag (`near`, `evm`, `bitcoin`, `solana`, `aptos`, `sui`, `zcash`, `starknet`, `ton`); the default `all` feature enables everything.

## Signing with NEAR MPC (chain signatures)

Every transaction type exposes `build_for_signing()` (the payload to hand to the MPC signer) and `build_with_signature(...)` (the broadcastable result). What those bytes are differs per chain:

| Chain | `build_for_signing()` returns | MPC domain | `build_with_signature(...)` returns |
|---|---|---|---|
| EVM | RLP preimage (caller keccak-256 hashes it) | secp256k1 (32-byte hash) | raw RLP tx for `eth_sendRawTransaction` |
| Bitcoin | legacy/segwit preimage (caller double-SHA256 hashes it) | secp256k1 (32-byte hash) | raw tx for `sendrawtransaction` |
| Zcash | final 32-byte ZIP-244 sighash (per input) | secp256k1 (sign directly) | raw v5 tx for `sendrawtransaction` |
| Starknet | 32-byte invoke-v3 tx-hash felt (big-endian) | account-defined (account abstraction) | `starknet_addInvokeTransaction` JSON body |
| NEAR | Borsh preimage (caller SHA-256 hashes it) | ed25519 or secp256k1 | Borsh-serialized `SignedTransaction` |
| Solana | serialized message bytes (sign as-is, never hash) | ed25519 (arbitrary bytes) | raw signed tx, base64 it for `sendTransaction` |
| Aptos | `sha3_256("APTOS::RawTransaction") ‖ BCS(raw_txn)` (sign as-is) | ed25519 (arbitrary bytes) | BCS `SignedTransaction` for `/transactions` (BCS content type) |
| Sui | 32-byte blake2b-256 of the intent message (sign as-is) | ed25519 (arbitrary bytes) | tx bytes + serialized signature for `executeTransactionBlock` |
| TON | 32-byte cell hash of the unsigned wallet body (sign as-is) | ed25519 (arbitrary bytes) | BoC of the signed external message (base64 helper included) |

## Installation

```toml
[dependencies]
omni-transaction = "0.3"
```

## Examples

For a complete set of examples see the [examples](https://github.com/Omni-rs/examples.git) repository.

### NEAR Types

The library provides safe wrappers for NEAR gas and token amounts:

```rust
use omni_transaction::near::types::{NearGas, NearToken};

// Creating NearToken amounts
let one_near = NearToken::from_near(1);           // 1 NEAR
let millinear = NearToken::from_millinear(500);   // 0.5 NEAR
let yoctonear = NearToken::from_yoctonear(1000);  // 1000 yoctoNEAR

// Creating NearGas amounts
let tgas = NearGas::from_tgas(100);               // 100 TGas
let gas = NearGas::from_gas(100_000_000_000);     // 100 TGas
```

Building a NEAR transaction:
```rust
use omni_transaction::{TransactionBuilder, TxBuilder, NEAR};
use omni_transaction::near::types::{Action, NearGas, NearToken, TransferAction, U64};
use omni_transaction::near::utils::PublicKeyStrExt;

let signer_id = "alice.near";
let signer_public_key = "ed25519:6E8sCci9badyRkXb3JoRpBj5p8C6Tw41ELDZoiihKEtp";
let nonce = U64(0);
let receiver_id = "bob.near";
let block_hash_str = "4reLvkAWfqk5fsqio1KLudk46cqRz9erQdaHkWZKMJDZ";
let transfer_action = Action::Transfer(TransferAction { 
    deposit: NearToken::from_near(1) 
});
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

Building a NEAR function call with gas and deposit:
```rust
use omni_transaction::near::types::{Action, FunctionCallAction, NearGas, NearToken};

let function_call = Action::FunctionCall(Box::new(FunctionCallAction {
    method_name: "my_method".to_string(),
    args: vec![],
    gas: NearGas::from_tgas(100),              // 100 TGas
    deposit: NearToken::from_near(1),          // 1 NEAR
}));
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

Building a Solana transaction (also works for Fogo and other SVM chains):

```rust
use omni_transaction::{TransactionBuilder, TxBuilder, SOLANA};
use omni_transaction::solana::types::{AccountMeta, Blockhash, Instruction, SolanaAddress};

let payer = SolanaAddress::from_base58("4vJ9JU1bJJE96FWSJKvHsmmFADCg4gpZQff4P3bkLKi").unwrap();
let to = SolanaAddress::from_base58("8opHzTAnfzRpPEx21XtnrVTX28YQuCpAjcn1PczScKh").unwrap();
let system_program = SolanaAddress([0u8; 32]);

// System-program transfer of 1_000_000 lamports.
let transfer = Instruction {
    program_id: system_program,
    accounts: vec![
        AccountMeta { pubkey: payer, is_signer: true, is_writable: true },
        AccountMeta { pubkey: to, is_signer: false, is_writable: true },
    ],
    data: [2u32.to_le_bytes().as_slice(), &1_000_000u64.to_le_bytes()].concat(),
};

let solana_tx = TransactionBuilder::new::<SOLANA>()
    .payer(payer)
    .instructions(vec![transfer])
    .recent_blockhash(Blockhash([0u8; 32])) // fetch a real one via RPC
    .build();

// The MPC ed25519 key signs these exact bytes (do NOT hash them).
let payload = solana_tx.build_for_signing();
// let signed = solana_tx.build_with_signature(&[signature]); // base64 -> sendTransaction
```

Building an Aptos transaction:

```rust
use omni_transaction::{TransactionBuilder, TxBuilder, APTOS};
use omni_transaction::aptos::types::{
    AccountAddress, EntryFunction, Identifier, ModuleId, TransactionPayload,
};

let receiver = AccountAddress::from_hex("0xdd").unwrap();

let aptos_tx = TransactionBuilder::new::<APTOS>()
    .sender(AccountAddress::from_hex("0xa550c18").unwrap())
    .sequence_number(0)
    .payload(TransactionPayload::EntryFunction(EntryFunction::new(
        ModuleId::new(AccountAddress::ONE, Identifier::new("aptos_account").unwrap()),
        Identifier::new("transfer").unwrap(),
        vec![],
        vec![receiver.as_bytes().to_vec(), 1_000u64.to_le_bytes().to_vec()],
    )))
    .max_gas_amount(2000)
    .gas_unit_price(100)
    .expiration_timestamp_secs(1_735_689_600)
    .chain_id(1) // mainnet
    .build();

// sha3_256("APTOS::RawTransaction") || BCS(raw_txn) — the MPC ed25519 key signs these bytes.
let signing_message = aptos_tx.build_for_signing();
// let signed = aptos_tx.build_with_signature(&public_key, &signature); // BCS body for POST /transactions
```

Building a Sui programmable transaction:

```rust
use omni_transaction::{TransactionBuilder, TxBuilder, SUI};
use omni_transaction::sui::types::{Argument, CallArg, Command, ObjectDigest, ObjectRef};
use omni_transaction::sui::utils::parse_sui_address;

let sender = parse_sui_address("0x2");
let recipient = parse_sui_address("0x3");

let sui_tx = TransactionBuilder::new::<SUI>()
    .sender(sender)
    .programmable(
        vec![CallArg::pure_u64(1_000_000), CallArg::pure_address(recipient)],
        vec![
            Command::SplitCoins { coin: Argument::GasCoin, amounts: vec![Argument::Input(0)] },
            Command::TransferObjects { objects: vec![Argument::Result(0)], address: Argument::Input(1) },
        ],
    )
    .gas_payment(vec![ObjectRef::new(parse_sui_address("0x1"), 2, ObjectDigest::new([0x63u8; 32]))])
    .gas_price(1000)
    .gas_budget(5_000_000)
    .build();

// 32-byte blake2b-256 digest of the intent message — the MPC ed25519 key signs it as-is.
let digest = sui_tx.build_for_signing();
// executeTransactionBlock takes base64(sui_tx.tx_bytes()) plus the serialized signature.
```

Building a Zcash transparent transaction (v5, ZIP-244):

```rust
use omni_transaction::{TransactionBuilder, TxBuilder, ZCASH};
use omni_transaction::zcash::types::{
    Amount, ConsensusBranchId, Hash, OutPoint, ScriptBuf, Sequence, SpentUtxo, TxIn, TxOut,
    Txid, Witness, ZcashSighashType,
};

let zcash_tx = TransactionBuilder::new::<ZCASH>()
    .consensus_branch_id(ConsensusBranchId::Nu6_3)
    .lock_time(0)
    .expiry_height(2_500_000)
    .inputs(vec![TxIn {
        previous_output: OutPoint { txid: Txid(Hash::all_zeros()), vout: 0 },
        script_sig: ScriptBuf::default(),
        sequence: Sequence::ENABLE_LOCKTIME_NO_RBF,
        witness: Witness::default(), // unused by Zcash, must stay empty
    }])
    .outputs(vec![TxOut {
        value: Amount::from_sat(99_990_000),
        script_pubkey: ScriptBuf::from_hex("76a9148132712c3ff19f3a151234616777420a6d7ef22688ac").unwrap(),
    }])
    .build();

// ZIP-244 commits to the value + scriptPubKey of every spent coin.
let spent = vec![SpentUtxo::new(
    100_000_000,
    ScriptBuf::from_hex("76a914507173527b4c3318a2aecd793bf1cfed705950cf88ac").unwrap(),
)];

// The final 32-byte sighash — the MPC secp256k1 key signs it directly.
let sighash = zcash_tx.build_for_signing(0, ZcashSighashType::All, &spent);
```

Building a Starknet invoke-v3 transaction:

```rust
use omni_transaction::{TransactionBuilder, TxBuilder, STARKNET};
use omni_transaction::starknet::types::{Call, Felt, ResourceBounds};
use omni_transaction::starknet::utils::{get_selector_from_name, CHAIN_ID_SEPOLIA};

let calls = vec![Call {
    to: Felt::from_hex("0x4138fd51f90d171df37e9d4419c8cdb67d525840c58f8a5c347be93a1c5277d").unwrap(),
    selector: get_selector_from_name("transfer"),
    calldata: vec![],
}];

let starknet_tx = TransactionBuilder::new::<STARKNET>()
    .chain_id(CHAIN_ID_SEPOLIA)
    .sender_address(Felt::from_hex("0x745d525a3582e91299d8d7c71730ffc4b1f191f5b219d800334bc0edad0983b").unwrap())
    .nonce(Felt::from_hex("0x9803").unwrap())
    .calls(&calls) // SNIP-6 __execute__ calldata encoding
    .l1_gas(ResourceBounds::new(0x186a0, 0x2d79883d20000))
    .l2_gas(ResourceBounds::new(0x5f5e100, 0xba43b7400))
    .l1_data_gas(ResourceBounds::new(0x186a0, 0x2d79883d20000))
    .build();

// The 32-byte transaction-hash felt; the account contract defines the signature scheme.
let tx_hash = starknet_tx.build_for_signing();
// build_with_signature(...) emits the starknet_addInvokeTransaction JSON body.
```

Building a TON transfer (wallet v5r1 by default, v4r2 also supported):

```rust
use omni_transaction::{TransactionBuilder, TxBuilder, TON};
use omni_transaction::ton::types::{Coins, InternalMessage, TonAddress};

let dest: TonAddress = "EQCD39VS5jcptHL8vMjEXrzGaRcCVYto7HUn4bpAOg8xqB2N"
    .parse()
    .unwrap(); // raw "0:hex" strings are accepted too

let public_key: [u8; 32] = mpc_derived_ed25519_public_key;

let ton_tx = TransactionBuilder::new::<TON>()
    .public_key(public_key)
    .seqno(0)
    .valid_until(1_756_200_000) // unix seconds
    .add_message(InternalMessage::new(dest, Coins(1_000_000_000)).with_comment("hi").unwrap())
    .deploy(true) // include StateInit while the wallet is not deployed yet
    .build();

// The 32-byte hash of the unsigned wallet body — the MPC ed25519 key signs it as-is.
let payload = ton_tx.build_for_signing();
// let boc = ton_tx.build_with_signature(signature); // 64-byte sig; base64 -> toncenter sendBoc
```
