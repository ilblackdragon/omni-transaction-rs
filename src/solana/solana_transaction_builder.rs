//! Transaction builder for Solana transactions
use std::collections::BTreeMap;

use super::solana_transaction::SolanaTransaction;
use super::types::{
    AddressLookupTableAccount, Blockhash, CompiledInstruction, Instruction,
    MessageAddressTableLookup, MessageHeader, SolanaAddress, SolanaMessage,
};
use crate::transaction_builder::TxBuilder;

/// Builder for [`SolanaTransaction`].
///
/// Compiles uncompiled [`Instruction`]s into a legacy message — or a
/// versioned V0 message when [`Self::address_lookup_tables`] is supplied —
/// reproducing the official SDK compiler byte-for-byte: account flags are
/// OR-ed across duplicate keys, the fee payer always comes first as a
/// writable signer, and the remaining keys are emitted in four buckets
/// (writable signers, read-only signers, writable non-signers, read-only
/// non-signers), each ordered lexicographically by the raw key bytes.
///
/// For V0 messages, a key is demoted from the static section to an address
/// table lookup only if it is not a signer, not an invoked program id, and
/// appears in one of the supplied tables.
pub struct SolanaTransactionBuilder {
    payer: Option<SolanaAddress>,
    instructions: Option<Vec<Instruction>>,
    recent_blockhash: Option<Blockhash>,
    address_lookup_tables: Option<Vec<AddressLookupTableAccount>>,
}

impl Default for SolanaTransactionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl TxBuilder<SolanaTransaction> for SolanaTransactionBuilder {
    /// Compiles the instructions into a [`SolanaTransaction`].
    ///
    /// # Panics
    ///
    /// Panics if `payer` or `recent_blockhash` was not set, if the compiled
    /// message would need an account or lookup-table index larger than a
    /// `u8` (more than 256 keys), or if the signed transaction would exceed
    /// [`PACKET_DATA_SIZE`](crate::solana::constants::PACKET_DATA_SIZE)
    /// (1232) bytes — such a transaction is unbroadcastable, and rejecting it
    /// at compile time keeps the caller from paying for a signature it could
    /// never use.
    fn build(&self) -> SolanaTransaction {
        let payer = self.payer.expect("payer is mandatory");
        let recent_blockhash = self
            .recent_blockhash
            .expect("recent_blockhash is mandatory");
        let instructions = self.instructions.clone().unwrap_or_default();
        let lookup_tables = self.address_lookup_tables.clone().unwrap_or_default();

        let transaction = SolanaTransaction {
            message: compile_message(payer, &instructions, recent_blockhash, &lookup_tables),
        };
        // Free size check: the same limit `build_for_signing` enforces, but
        // raised before any signing round trip is paid for.
        transaction.assert_within_packet_limit();
        transaction
    }
}

impl SolanaTransactionBuilder {
    /// Creates a new, empty builder.
    pub const fn new() -> Self {
        Self {
            payer: None,
            instructions: None,
            recent_blockhash: None,
            address_lookup_tables: None,
        }
    }

    /// Fee payer of the transaction (mandatory): always compiled as the
    /// first account key, forced to signer + writable. Must be the ed25519
    /// public key whose signature will be supplied.
    pub const fn payer(mut self, payer: SolanaAddress) -> Self {
        self.payer = Some(payer);
        self
    }

    /// Instructions of the transaction, in execution order.
    pub fn instructions(mut self, instructions: Vec<Instruction>) -> Self {
        self.instructions = Some(instructions);
        self
    }

    /// A recent blockhash (mandatory). Note that a blockhash expires after
    /// ~150 slots (~60-90 s); for slow signing round trips (e.g. via MPC)
    /// consider durable nonces.
    pub const fn recent_blockhash(mut self, recent_blockhash: Blockhash) -> Self {
        self.recent_blockhash = Some(recent_blockhash);
        self
    }

    /// Address lookup tables available for compilation. Supplying a
    /// non-empty list makes the builder compile a versioned V0 message
    /// (demoting eligible keys into table lookups); otherwise a legacy
    /// message is compiled.
    pub fn address_lookup_tables(
        mut self,
        address_lookup_tables: Vec<AddressLookupTableAccount>,
    ) -> Self {
        self.address_lookup_tables = Some(address_lookup_tables);
        self
    }
}

/// Per-key flags accumulated across all instructions, mirroring the SDK's
/// `CompiledKeyMeta` (solana-message `compiled_keys.rs`).
#[derive(Debug, Default, Clone, Copy)]
struct CompiledKeyMeta {
    is_signer: bool,
    is_writable: bool,
    is_invoked: bool,
}

/// Compiles the message, choosing legacy or V0 based on whether lookup
/// tables were supplied.
fn compile_message(
    payer: SolanaAddress,
    instructions: &[Instruction],
    recent_blockhash: Blockhash,
    lookup_tables: &[AddressLookupTableAccount],
) -> SolanaMessage {
    let mut key_meta_map = compile_key_metas(payer, instructions);

    if lookup_tables.is_empty() {
        let (header, account_keys) = split_into_buckets(payer, &key_meta_map);
        let instructions = compile_instructions(instructions, &account_keys);
        return SolanaMessage::Legacy {
            header,
            account_keys,
            recent_blockhash,
            instructions,
        };
    }

    // V0: demote eligible keys into address table lookups. The combined
    // runtime index space is: static keys, then every writable lookup
    // address (in table order), then every read-only lookup address.
    let mut address_table_lookups = Vec::with_capacity(lookup_tables.len());
    let mut loaded_writable_keys = Vec::new();
    let mut loaded_readonly_keys = Vec::new();
    for table in lookup_tables {
        if let Some((lookup, writable_keys, readonly_keys)) =
            try_extract_table_lookup(&mut key_meta_map, table)
        {
            address_table_lookups.push(lookup);
            loaded_writable_keys.extend(writable_keys);
            loaded_readonly_keys.extend(readonly_keys);
        }
    }

    let (header, account_keys) = split_into_buckets(payer, &key_meta_map);
    let mut combined_keys = account_keys.clone();
    combined_keys.extend(loaded_writable_keys);
    combined_keys.extend(loaded_readonly_keys);
    let instructions = compile_instructions(instructions, &combined_keys);

    SolanaMessage::V0 {
        header,
        account_keys,
        recent_blockhash,
        instructions,
        address_table_lookups,
    }
}

/// Collects every instruction program id and account into a `BTreeMap`
/// (deduplicating and OR-ing flags), then forces the fee payer to
/// signer + writable — exactly like the SDK's `CompiledKeys::compile`.
fn compile_key_metas(
    payer: SolanaAddress,
    instructions: &[Instruction],
) -> BTreeMap<SolanaAddress, CompiledKeyMeta> {
    let mut key_meta_map = BTreeMap::<SolanaAddress, CompiledKeyMeta>::new();
    for instruction in instructions {
        key_meta_map
            .entry(instruction.program_id)
            .or_default()
            .is_invoked = true;
        for account in &instruction.accounts {
            let meta = key_meta_map.entry(account.pubkey).or_default();
            meta.is_signer |= account.is_signer;
            meta.is_writable |= account.is_writable;
        }
    }
    let payer_meta = key_meta_map.entry(payer).or_default();
    payer_meta.is_signer = true;
    payer_meta.is_writable = true;
    key_meta_map
}

/// Emits the account keys in the canonical bucket order — payer first, then
/// remaining writable signers, read-only signers, writable non-signers,
/// read-only non-signers, each bucket lexicographically ascending (the
/// `BTreeMap` iteration order) — and derives the message header counts.
fn split_into_buckets(
    payer: SolanaAddress,
    key_meta_map: &BTreeMap<SolanaAddress, CompiledKeyMeta>,
) -> (MessageHeader, Vec<SolanaAddress>) {
    let mut writable_signer_keys = vec![payer];
    let mut readonly_signer_keys = Vec::new();
    let mut writable_non_signer_keys = Vec::new();
    let mut readonly_non_signer_keys = Vec::new();
    for (&key, meta) in key_meta_map {
        if key == payer {
            continue; // Already placed first (forced signer + writable).
        }
        match (meta.is_signer, meta.is_writable) {
            (true, true) => writable_signer_keys.push(key),
            (true, false) => readonly_signer_keys.push(key),
            (false, true) => writable_non_signer_keys.push(key),
            (false, false) => readonly_non_signer_keys.push(key),
        }
    }

    let header = MessageHeader {
        num_required_signatures: to_u8(
            writable_signer_keys.len() + readonly_signer_keys.len(),
            "number of required signatures",
        ),
        num_readonly_signed_accounts: to_u8(
            readonly_signer_keys.len(),
            "number of read-only signed accounts",
        ),
        num_readonly_unsigned_accounts: to_u8(
            readonly_non_signer_keys.len(),
            "number of read-only unsigned accounts",
        ),
    };

    let mut account_keys = writable_signer_keys;
    account_keys.extend(readonly_signer_keys);
    account_keys.extend(writable_non_signer_keys);
    account_keys.extend(readonly_non_signer_keys);
    (header, account_keys)
}

/// Extracts one address table lookup from the key metas, draining the
/// matching keys — the SDK's `CompiledKeys::try_extract_table_lookup`.
/// Returns `None` when the table matches no key.
fn try_extract_table_lookup(
    key_meta_map: &mut BTreeMap<SolanaAddress, CompiledKeyMeta>,
    table: &AddressLookupTableAccount,
) -> Option<(
    MessageAddressTableLookup,
    Vec<SolanaAddress>,
    Vec<SolanaAddress>,
)> {
    let (writable_indexes, drained_writable_keys) =
        drain_keys_found_in_lookup_table(key_meta_map, &table.addresses, |meta| {
            !meta.is_signer && !meta.is_invoked && meta.is_writable
        });
    let (readonly_indexes, drained_readonly_keys) =
        drain_keys_found_in_lookup_table(key_meta_map, &table.addresses, |meta| {
            !meta.is_signer && !meta.is_invoked && !meta.is_writable
        });
    if writable_indexes.is_empty() && readonly_indexes.is_empty() {
        return None;
    }
    Some((
        MessageAddressTableLookup {
            account_key: table.key,
            writable_indexes,
            readonly_indexes,
        },
        drained_writable_keys,
        drained_readonly_keys,
    ))
}

/// Removes from the metas every key that passes `key_meta_filter` and
/// appears in `table_addresses`, returning the table indexes and the
/// drained keys (in lexicographic key order, matching the SDK).
fn drain_keys_found_in_lookup_table(
    key_meta_map: &mut BTreeMap<SolanaAddress, CompiledKeyMeta>,
    table_addresses: &[SolanaAddress],
    key_meta_filter: impl Fn(&CompiledKeyMeta) -> bool,
) -> (Vec<u8>, Vec<SolanaAddress>) {
    let mut lookup_table_indexes = Vec::new();
    let mut drained_keys = Vec::new();
    for (&key, meta) in key_meta_map.iter() {
        if !key_meta_filter(meta) {
            continue;
        }
        if let Some(position) = table_addresses.iter().position(|address| *address == key) {
            lookup_table_indexes.push(to_u8(position, "lookup table index"));
            drained_keys.push(key);
        }
    }
    for key in &drained_keys {
        key_meta_map.remove(key);
    }
    (lookup_table_indexes, drained_keys)
}

/// Compiles instructions by replacing each key with its index in
/// `account_keys` (for V0: the combined static + lookup address space).
fn compile_instructions(
    instructions: &[Instruction],
    account_keys: &[SolanaAddress],
) -> Vec<CompiledInstruction> {
    instructions
        .iter()
        .map(|instruction| CompiledInstruction {
            program_id_index: position_of(account_keys, &instruction.program_id),
            accounts: instruction
                .accounts
                .iter()
                .map(|meta| position_of(account_keys, &meta.pubkey))
                .collect(),
            data: instruction.data.clone(),
        })
        .collect()
}

/// Index of `key` in `account_keys` as a `u8`.
fn position_of(account_keys: &[SolanaAddress], key: &SolanaAddress) -> u8 {
    let position = account_keys
        .iter()
        .position(|account_key| account_key == key)
        .expect("instruction key missing from compiled account keys");
    to_u8(position, "account index")
}

/// Converts a count/index to `u8`, panicking with a descriptive message on
/// overflow (Solana account and table indexes are single bytes).
fn to_u8(value: usize, what: &str) -> u8 {
    u8::try_from(value).unwrap_or_else(|_| panic!("{what} must fit in a u8 (max 255), got {value}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solana::types::AccountMeta;
    use crate::solana::utils::system_transfer;

    /// Payer of spec vectors V2-V4: the ed25519 public key of seed `[1; 32]`
    /// (`AKnL4NNf3DGWZJS6cPknBuEGnVsV4A4m5tgebLHaRSZ9`).
    const PAYER_HEX: &str = "8a88e3dd7409f195fd52db2d3cba5d72ca6709bf1d94121bf3748801b40f6f5c";
    /// Memo program (`MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr`).
    const MEMO_HEX: &str = "054a535a992921064d24e87160da387c7c35b5ddbc92bb81e41fa8404105448d";
    /// Rent sysvar (`SysvarRent111111111111111111111111111111111`).
    const RENT_HEX: &str = "06a7d517192c5c51218cc94c3d4af17f58daee089ba1fd44e3dbd98a00000000";
    /// Blockhash of spec vectors V2-V4
    /// (`EETubP5AKHgjPAhzPAFcb8BAY1hMH639CWCFTqi3hq1k`).
    const BLOCKHASH_HEX: &str = "c49ae77603782054f17a9decea43b444eba0edb12c6f1d31c6e0e4a84bf052eb";

    const V1_MESSAGE_HEX: &str = "01000103010101010101010101010101010101010101010101010101010101010101010102000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001020200010c0200000040420f0000000000";
    const V2_MESSAGE_HEX: &str = "010001038a88e3dd7409f195fd52db2d3cba5d72ca6709bf1d94121bf3748801b40f6f5c02000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000c49ae77603782054f17a9decea43b444eba0edb12c6f1d31c6e0e4a84bf052eb01020200010c020000002a00000000000000";
    const V3_MESSAGE_HEX: &str = "010003058a88e3dd7409f195fd52db2d3cba5d72ca6709bf1d94121bf3748801b40f6f5c02000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000054a535a992921064d24e87160da387c7c35b5ddbc92bb81e41fa8404105448d06a7d517192c5c51218cc94c3d4af17f58daee089ba1fd44e3dbd98a00000000c49ae77603782054f17a9decea43b444eba0edb12c6f1d31c6e0e4a84bf052eb02020200010c0200000007000000000000000302040004deadbeef";
    const V4_MESSAGE_HEX: &str = "80010001028a88e3dd7409f195fd52db2d3cba5d72ca6709bf1d94121bf3748801b40f6f5c054a535a992921064d24e87160da387c7c35b5ddbc92bb81e41fa8404105448dc49ae77603782054f17a9decea43b444eba0edb12c6f1d31c6e0e4a84bf052eb0101030203000301020301d09e258ae6cf647b0e2441b43818bb9713c47f6351f87bef581f6f8346c9cc2a01000101";

    fn address(hex_str: &str) -> SolanaAddress {
        SolanaAddress(hex::decode(hex_str).unwrap().try_into().unwrap())
    }

    fn to_address() -> SolanaAddress {
        address("0200000000000000000000000000000000000000000000000000000000000000")
    }

    fn blockhash() -> Blockhash {
        Blockhash(hex::decode(BLOCKHASH_HEX).unwrap().try_into().unwrap())
    }

    /// Spec vector V1: legacy System transfer, all-zero blockhash.
    #[test]
    fn test_builder_v1_legacy_transfer_matches_official_vector() {
        let from = SolanaAddress([0x01; 32]);
        let tx = SolanaTransactionBuilder::new()
            .payer(from)
            .instructions(vec![system_transfer(from, to_address(), 1_000_000)])
            .recent_blockhash(Blockhash([0u8; 32]))
            .build();

        assert_eq!(hex::encode(tx.build_for_signing()), V1_MESSAGE_HEX);
    }

    /// Spec vector V2: legacy transfer of 42 lamports from the seed-`[1; 32]`
    /// keypair's address.
    #[test]
    fn test_builder_v2_legacy_transfer_matches_official_vector() {
        let payer = address(PAYER_HEX);
        let tx = SolanaTransactionBuilder::new()
            .payer(payer)
            .instructions(vec![system_transfer(payer, to_address(), 42)])
            .recent_blockhash(blockhash())
            .build();

        assert_eq!(hex::encode(tx.build_for_signing()), V2_MESSAGE_HEX);
    }

    /// Spec vector V3 — the account-ordering oracle: proves dedup of the
    /// payer (a signer in both instructions), OR-ing of flags, header
    /// derivation (1, 0, 3) and the lexicographic order of the read-only
    /// non-signer bucket: system (00..) < memo (05..) < rent (06..),
    /// regardless of instruction order.
    #[test]
    fn test_builder_v3_account_ordering_matches_official_vector() {
        let payer = address(PAYER_HEX);
        let memo_instruction = Instruction {
            program_id: address(MEMO_HEX),
            accounts: vec![
                AccountMeta::new_readonly(address(RENT_HEX), false),
                AccountMeta::new_readonly(payer, true),
            ],
            data: vec![0xde, 0xad, 0xbe, 0xef],
        };
        let tx = SolanaTransactionBuilder::new()
            .payer(payer)
            .instructions(vec![
                system_transfer(payer, to_address(), 7),
                memo_instruction,
            ])
            .recent_blockhash(blockhash())
            .build();

        assert_eq!(
            *tx.message.header(),
            MessageHeader {
                num_required_signatures: 1,
                num_readonly_signed_accounts: 0,
                num_readonly_unsigned_accounts: 3,
            }
        );
        assert_eq!(hex::encode(tx.build_for_signing()), V3_MESSAGE_HEX);
    }

    /// Spec vector V4: V0 message with one lookup table — one writable and
    /// one read-only address demoted, while the invoked memo program and the
    /// payer stay in the static section.
    #[test]
    fn test_builder_v4_v0_lookup_table_matches_official_vector() {
        let payer = address(PAYER_HEX);
        let extra1 = address("e22f250b3cf697762594fe87b7ff35124991842befacf8472307ddc211f619e7");
        let extra2 = address("826c4e255256d9d0d4898979d2b434eafe22f36bfeb95b2a0c4a9db8ef0fa91d");
        let table = AddressLookupTableAccount {
            key: address("d09e258ae6cf647b0e2441b43818bb9713c47f6351f87bef581f6f8346c9cc2a"),
            addresses: vec![extra1, extra2],
        };
        let instruction = Instruction {
            program_id: address(MEMO_HEX),
            accounts: vec![
                AccountMeta::new(extra1, false),
                AccountMeta::new_readonly(extra2, false),
                AccountMeta::new(payer, true),
            ],
            data: vec![1, 2, 3],
        };

        let tx = SolanaTransactionBuilder::new()
            .payer(payer)
            .instructions(vec![instruction])
            .recent_blockhash(blockhash())
            .address_lookup_tables(vec![table])
            .build();

        // Static keys keep only the payer and the invoked program.
        assert_eq!(*tx.message.account_keys(), vec![payer, address(MEMO_HEX)]);
        assert_eq!(hex::encode(tx.build_for_signing()), V4_MESSAGE_HEX);
    }

    /// An empty lookup-table list compiles a legacy message; a supplied
    /// table that matches nothing still compiles a V0 message (with no
    /// lookups), mirroring `v0::Message::try_compile`.
    #[test]
    fn test_builder_message_version_selection() {
        let payer = address(PAYER_HEX);
        let build = |tables: Option<Vec<AddressLookupTableAccount>>| {
            let mut builder = SolanaTransactionBuilder::new()
                .payer(payer)
                .instructions(vec![system_transfer(payer, to_address(), 1)])
                .recent_blockhash(blockhash());
            if let Some(tables) = tables {
                builder = builder.address_lookup_tables(tables);
            }
            builder.build()
        };

        assert!(matches!(build(None).message, SolanaMessage::Legacy { .. }));
        let unrelated_table = AddressLookupTableAccount {
            key: address(MEMO_HEX),
            addresses: vec![address(RENT_HEX)],
        };
        let v0_tx = build(Some(vec![unrelated_table]));
        assert!(matches!(
            &v0_tx.message,
            SolanaMessage::V0 { address_table_lookups, .. } if address_table_lookups.is_empty()
        ));
    }

    #[test]
    #[should_panic(expected = "payer is mandatory")]
    fn test_builder_missing_payer_panics() {
        let _ = SolanaTransactionBuilder::new()
            .recent_blockhash(blockhash())
            .build();
    }

    #[test]
    #[should_panic(expected = "recent_blockhash is mandatory")]
    fn test_builder_missing_recent_blockhash_panics() {
        let _ = SolanaTransactionBuilder::new()
            .payer(address(PAYER_HEX))
            .build();
    }

    /// An instruction set whose signed transaction cannot fit in a packet is
    /// rejected at compile time, before any signature is requested.
    #[test]
    #[should_panic(expected = "packet limit")]
    fn test_builder_oversized_transaction_panics() {
        let payer = address(PAYER_HEX);
        let instruction = Instruction {
            program_id: address(MEMO_HEX),
            accounts: vec![],
            data: vec![0u8; 1300],
        };
        let _ = SolanaTransactionBuilder::new()
            .payer(payer)
            .instructions(vec![instruction])
            .recent_blockhash(blockhash())
            .build();
    }

    /// A transaction just below the limit still compiles and signs.
    #[test]
    fn test_builder_transaction_just_under_packet_limit_builds() {
        let payer = address(PAYER_HEX);
        let build_with_data_len = |len: usize| {
            SolanaTransactionBuilder::new()
                .payer(payer)
                .instructions(vec![Instruction {
                    program_id: address(MEMO_HEX),
                    accounts: vec![],
                    data: vec![0u8; len],
                }])
                .recent_blockhash(blockhash())
                .build()
        };

        // 1 signature (65 bytes) + 3 header + compact key count + 2 * 32 keys
        // + 32 blockhash + instruction overhead is 170 bytes, so 1060 data
        // bytes land 2 bytes below the limit.
        let tx = build_with_data_len(1060);
        assert_eq!(tx.signed_size(), 1230);
        assert!(tx.signed_size() <= crate::solana::constants::PACKET_DATA_SIZE);
        assert_eq!(
            tx.build_with_signature(&[crate::solana::types::SolanaSignature([0u8; 64])])
                .len(),
            tx.signed_size()
        );
    }

    /// More than 256 unique account keys cannot be indexed by a `u8`.
    #[test]
    #[should_panic(expected = "must fit in a u8")]
    fn test_builder_too_many_accounts_panics() {
        let payer = address(PAYER_HEX);
        let accounts = (0u16..300)
            .map(|i| {
                let mut bytes = [0xAAu8; 32];
                bytes[0] = (i >> 8) as u8;
                bytes[1] = (i & 0xff) as u8;
                AccountMeta::new_readonly(SolanaAddress(bytes), false)
            })
            .collect();
        let instruction = Instruction {
            program_id: address(MEMO_HEX),
            accounts,
            data: vec![],
        };
        let _ = SolanaTransactionBuilder::new()
            .payer(payer)
            .instructions(vec![instruction])
            .recent_blockhash(blockhash())
            .build();
    }
}
