#![cfg(feature = "solana")]
//! End-to-end tests comparing `omni-transaction`'s Solana implementation
//! byte-for-byte against the official Anza reference crates
//! (`solana-message`, `solana-transaction`, ...): builder ->
//! `build_for_signing` -> sign with `solana-keypair` (standing in for the
//! NEAR MPC ed25519 signer) -> `build_with_signature` -> wire bytes.

use std::str::FromStr;

use omni_transaction::solana::constants::PACKET_DATA_SIZE;
use omni_transaction::solana::types::{
    AccountMeta as OmniAccountMeta, AddressLookupTableAccount as OmniAddressLookupTableAccount,
    Blockhash, Instruction as OmniInstruction, SolanaAddress, SolanaSignature,
};
use omni_transaction::solana::SolanaTransaction;
use omni_transaction::{TransactionBuilder, TxBuilder, SOLANA};

use solana_hash::Hash;
use solana_instruction::{AccountMeta, Instruction};
use solana_keypair::Keypair;
use solana_message::{v0, AddressLookupTableAccount, Message, VersionedMessage};
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use solana_transaction::{versioned::VersionedTransaction, Transaction};

const BLOCKHASH: &str = "EETubP5AKHgjPAhzPAFcb8BAY1hMH639CWCFTqi3hq1k";
const TO: &str = "8opHzTAnfzRpPEx21XtnrVTX28YQuCpAjcn1PczScKh";
const MEMO_PROGRAM: &str = "MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr";
const RENT_SYSVAR: &str = "SysvarRent111111111111111111111111111111111";

const fn omni_address(pubkey: &Pubkey) -> SolanaAddress {
    SolanaAddress(pubkey.to_bytes())
}

fn omni_instruction(instruction: &Instruction) -> OmniInstruction {
    OmniInstruction {
        program_id: omni_address(&instruction.program_id),
        accounts: instruction
            .accounts
            .iter()
            .map(|meta| OmniAccountMeta {
                pubkey: omni_address(&meta.pubkey),
                is_signer: meta.is_signer,
                is_writable: meta.is_writable,
            })
            .collect(),
        data: instruction.data.clone(),
    }
}

fn build_ours(
    payer: &Pubkey,
    instructions: &[Instruction],
    lookup_tables: &[AddressLookupTableAccount],
) -> SolanaTransaction {
    let mut builder = TransactionBuilder::new::<SOLANA>()
        .payer(omni_address(payer))
        .instructions(instructions.iter().map(omni_instruction).collect())
        .recent_blockhash(Blockhash(Hash::from_str(BLOCKHASH).unwrap().to_bytes()));
    if !lookup_tables.is_empty() {
        builder = builder.address_lookup_tables(
            lookup_tables
                .iter()
                .map(|table| OmniAddressLookupTableAccount {
                    key: omni_address(&table.key),
                    addresses: table.addresses.iter().map(omni_address).collect(),
                })
                .collect(),
        );
    }
    builder.build()
}

fn omni_signature(signature: &impl AsRef<[u8]>) -> SolanaSignature {
    SolanaSignature(signature.as_ref().try_into().unwrap())
}

/// Legacy System transfer: message preimage, deterministic ed25519
/// signature and signed wire bytes all match the reference crates.
#[test]
fn test_legacy_transfer_round_trip_against_reference() {
    let keypair = Keypair::new_from_array([1u8; 32]);
    let payer = keypair.pubkey();
    let to = Pubkey::from_str(TO).unwrap();
    let blockhash = Hash::from_str(BLOCKHASH).unwrap();

    let instruction = solana_system_interface::instruction::transfer(&payer, &to, 42);
    let reference_message =
        Message::new_with_blockhash(std::slice::from_ref(&instruction), Some(&payer), &blockhash);
    let reference_message_bytes = bincode::serialize(&reference_message).unwrap();

    let ours = build_ours(&payer, &[instruction], &[]);
    let payload = ours.build_for_signing();
    assert_eq!(payload, reference_message_bytes);

    // Sign the exact preimage (as the NEAR MPC ed25519 signer would).
    let signature = keypair.sign_message(&payload);
    let reference_wire = bincode::serialize(&Transaction {
        signatures: vec![signature],
        message: reference_message,
    })
    .unwrap();

    let our_wire = ours.build_with_signature(&[omni_signature(&signature)]);
    assert_eq!(our_wire, reference_wire);
}

/// Multi-instruction message with duplicated and read-only keys: account
/// dedup, flag OR-ing, header derivation and bucket ordering all match the
/// reference compiler.
#[test]
fn test_account_ordering_round_trip_against_reference() {
    let keypair = Keypair::new_from_array([1u8; 32]);
    let payer = keypair.pubkey();
    let to = Pubkey::from_str(TO).unwrap();
    let blockhash = Hash::from_str(BLOCKHASH).unwrap();

    let transfer = solana_system_interface::instruction::transfer(&payer, &to, 7);
    let memo = Instruction {
        program_id: Pubkey::from_str(MEMO_PROGRAM).unwrap(),
        accounts: vec![
            AccountMeta::new_readonly(Pubkey::from_str(RENT_SYSVAR).unwrap(), false),
            AccountMeta::new_readonly(payer, true),
        ],
        data: vec![0xde, 0xad, 0xbe, 0xef],
    };
    let instructions = vec![transfer, memo];

    let reference_message = Message::new_with_blockhash(&instructions, Some(&payer), &blockhash);
    let reference_message_bytes = bincode::serialize(&reference_message).unwrap();

    let ours = build_ours(&payer, &instructions, &[]);
    let payload = ours.build_for_signing();
    assert_eq!(payload, reference_message_bytes);

    let signature = keypair.sign_message(&payload);
    let reference_wire = bincode::serialize(&Transaction {
        signatures: vec![signature],
        message: reference_message,
    })
    .unwrap();
    assert_eq!(
        ours.build_with_signature(&[omni_signature(&signature)]),
        reference_wire
    );
}

/// V0 message with an address lookup table: version prefix, key demotion,
/// lookup section and the signed `VersionedTransaction` wire bytes all match
/// the reference crates.
#[test]
fn test_v0_lookup_table_round_trip_against_reference() {
    let keypair = Keypair::new_from_array([1u8; 32]);
    let payer = keypair.pubkey();
    let blockhash = Hash::from_str(BLOCKHASH).unwrap();

    let extra1 = Pubkey::from_str("GDvqQxaEFWkyroZE3J96BpUgJXUSLVDdmEUjJUKLGFNe").unwrap();
    let extra2 = Pubkey::from_str("9n7ncYQ9oLTfLPKmPxN6QoZDMYtQ6fFzKvpv1zonUyPz").unwrap();
    let table = AddressLookupTableAccount {
        key: Pubkey::from_str("F3MfgEJe1TApJiA14nN2m4uAH4EBVrqdBnHeGeSXvQ7B").unwrap(),
        addresses: vec![extra1, extra2],
    };
    let instruction = Instruction {
        program_id: Pubkey::from_str(MEMO_PROGRAM).unwrap(),
        accounts: vec![
            AccountMeta::new(extra1, false),
            AccountMeta::new_readonly(extra2, false),
            AccountMeta::new(payer, true),
        ],
        data: vec![1, 2, 3],
    };

    let reference_message = VersionedMessage::V0(
        v0::Message::try_compile(
            &payer,
            std::slice::from_ref(&instruction),
            std::slice::from_ref(&table),
            blockhash,
        )
        .unwrap(),
    );
    let reference_message_bytes = bincode::serialize(&reference_message).unwrap();

    let ours = build_ours(&payer, &[instruction], &[table]);
    let payload = ours.build_for_signing();
    assert_eq!(payload, reference_message_bytes);

    let signature = keypair.sign_message(&payload);
    let reference_wire = bincode::serialize(&VersionedTransaction {
        signatures: vec![signature],
        message: reference_message,
    })
    .unwrap();
    assert_eq!(
        ours.build_with_signature(&[omni_signature(&signature)]),
        reference_wire
    );
}

/// Builds a memo-only reference transaction with `data_len` instruction bytes
/// and returns its signed wire length, as the official crates compute it.
fn reference_signed_len(keypair: &Keypair, data_len: usize) -> usize {
    let payer = keypair.pubkey();
    let memo = Instruction {
        program_id: Pubkey::from_str(MEMO_PROGRAM).unwrap(),
        accounts: vec![],
        data: vec![0u8; data_len],
    };
    let message = Message::new_with_blockhash(
        std::slice::from_ref(&memo),
        Some(&payer),
        &Hash::from_str(BLOCKHASH).unwrap(),
    );
    let signature = keypair.sign_message(&bincode::serialize(&message).unwrap());
    bincode::serialize(&Transaction {
        signatures: vec![signature],
        message,
    })
    .unwrap()
    .len()
}

/// The 1232-byte packet limit is enforced while the message is compiled —
/// i.e. before any signature is requested — so a NEAR contract never pays for
/// an MPC signature it could not broadcast.
#[test]
fn test_oversized_transaction_is_rejected_before_signing() {
    let keypair = Keypair::new_from_array([1u8; 32]);
    let payer = keypair.pubkey();
    let memo = Instruction {
        program_id: Pubkey::from_str(MEMO_PROGRAM).unwrap(),
        accounts: vec![],
        data: vec![0u8; 1300],
    };
    // The reference crates build this unbroadcastable transaction happily.
    assert!(reference_signed_len(&keypair, 1300) > PACKET_DATA_SIZE);

    let previous_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let build = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        build_ours(&payer, std::slice::from_ref(&memo), &[])
    }));
    std::panic::set_hook(previous_hook);

    let payload = build.expect_err("an oversized transaction must be rejected at build time");
    let panic_message = payload
        .downcast_ref::<String>()
        .cloned()
        .unwrap_or_else(|| String::from("<non-string panic>"));
    assert!(
        panic_message.contains("packet limit"),
        "unexpected panic message: {panic_message}"
    );
}

/// A transaction sitting exactly on the packet limit still builds, signs and
/// matches the reference wire bytes byte-for-byte — the limit check must not
/// be off by the signature bytes in either direction.
#[test]
fn test_transaction_at_packet_limit_round_trips_against_reference() {
    let keypair = Keypair::new_from_array([1u8; 32]);
    let payer = keypair.pubkey();

    // Largest memo payload the reference crates still fit in one packet.
    let data_len = (0..1300)
        .rev()
        .find(|&len| reference_signed_len(&keypair, len) <= PACKET_DATA_SIZE)
        .expect("some payload must fit");
    assert_eq!(reference_signed_len(&keypair, data_len), PACKET_DATA_SIZE);
    assert!(reference_signed_len(&keypair, data_len + 1) > PACKET_DATA_SIZE);

    let memo = Instruction {
        program_id: Pubkey::from_str(MEMO_PROGRAM).unwrap(),
        accounts: vec![],
        data: vec![0u8; data_len],
    };
    let reference_message = Message::new_with_blockhash(
        std::slice::from_ref(&memo),
        Some(&payer),
        &Hash::from_str(BLOCKHASH).unwrap(),
    );
    let reference_message_bytes = bincode::serialize(&reference_message).unwrap();

    let ours = build_ours(&payer, std::slice::from_ref(&memo), &[]);
    assert_eq!(ours.signed_size(), PACKET_DATA_SIZE);
    let payload = ours.build_for_signing();
    assert_eq!(payload, reference_message_bytes);

    let signature = keypair.sign_message(&payload);
    let reference_wire = bincode::serialize(&Transaction {
        signatures: vec![signature],
        message: reference_message,
    })
    .unwrap();
    let our_wire = ours.build_with_signature(&[omni_signature(&signature)]);
    assert_eq!(our_wire, reference_wire);
    assert_eq!(our_wire.len(), PACKET_DATA_SIZE);
}

/// Deterministic xorshift64 PRNG so the fuzz tests are reproducible without
/// a `rand` dependency.
struct XorShift64(u64);

impl XorShift64 {
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn next_index(&mut self, bound: usize) -> usize {
        (self.next_u64() % bound as u64) as usize
    }

    fn next_bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
}

fn fuzz_key_pool() -> (Vec<Pubkey>, Vec<Pubkey>) {
    let accounts = (0u8..8)
        .map(|i| Pubkey::from([i * 7 + 10; 32]))
        .collect::<Vec<_>>();
    let programs = vec![Pubkey::from([200u8; 32]), Pubkey::from([201u8; 32])];
    (accounts, programs)
}

fn random_instructions(
    rng: &mut XorShift64,
    accounts: &[Pubkey],
    programs: &[Pubkey],
) -> Vec<Instruction> {
    let instruction_count = 1 + rng.next_index(3);
    (0..instruction_count)
        .map(|_| {
            let account_count = rng.next_index(5);
            Instruction {
                program_id: programs[rng.next_index(programs.len())],
                accounts: (0..account_count)
                    .map(|_| AccountMeta {
                        pubkey: accounts[rng.next_index(accounts.len())],
                        is_signer: rng.next_bool(),
                        is_writable: rng.next_bool(),
                    })
                    .collect(),
                data: vec![rng.next_u64() as u8],
            }
        })
        .collect()
}

/// Randomized ordering fuzz: 1000 random instruction sets (random subsets of
/// 8 fixed keys with random signer/writable flags, random payer) must
/// compile to byte-identical legacy messages, pinning the BTreeMap
/// lexicographic bucket ordering and flag OR-ing.
#[test]
fn test_fuzz_legacy_compilation_parity_with_reference() {
    let blockhash = Hash::from_str(BLOCKHASH).unwrap();
    let (accounts, programs) = fuzz_key_pool();
    let mut rng = XorShift64(0x5eed_cafe_f00d_1234);

    for iteration in 0..1000 {
        let instructions = random_instructions(&mut rng, &accounts, &programs);
        let payer = accounts[rng.next_index(accounts.len())];

        let reference_message =
            Message::new_with_blockhash(&instructions, Some(&payer), &blockhash);
        let reference_bytes = bincode::serialize(&reference_message).unwrap();

        let ours = build_ours(&payer, &instructions, &[]);
        assert_eq!(
            ours.build_for_signing(),
            reference_bytes,
            "legacy compilation diverged from reference at iteration {iteration}"
        );
    }
}

/// Randomized V0 fuzz: same random instruction sets compiled against a
/// lookup table containing half of the key pool, pinning the demotion rules
/// (signers and invoked program ids stay static).
#[test]
fn test_fuzz_v0_compilation_parity_with_reference() {
    let blockhash = Hash::from_str(BLOCKHASH).unwrap();
    let (accounts, programs) = fuzz_key_pool();
    let table = AddressLookupTableAccount {
        key: Pubkey::from([222u8; 32]),
        addresses: accounts[..4].to_vec(),
    };
    let mut rng = XorShift64(0xdead_beef_0bad_5eed);

    for iteration in 0..1000 {
        let instructions = random_instructions(&mut rng, &accounts, &programs);
        let payer = accounts[rng.next_index(accounts.len())];

        let reference_message = VersionedMessage::V0(
            v0::Message::try_compile(
                &payer,
                &instructions,
                std::slice::from_ref(&table),
                blockhash.clone(),
            )
            .unwrap(),
        );
        let reference_bytes = bincode::serialize(&reference_message).unwrap();

        let ours = build_ours(&payer, &instructions, std::slice::from_ref(&table));
        assert_eq!(
            ours.build_for_signing(),
            reference_bytes,
            "v0 compilation diverged from reference at iteration {iteration}"
        );
    }
}
