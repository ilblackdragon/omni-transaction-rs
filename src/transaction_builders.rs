//! Low level transaction builders for different blockchains.
#[cfg(feature = "aptos")]
use crate::aptos::AptosTransactionBuilder;

#[cfg(feature = "bitcoin")]
use crate::bitcoin::BitcoinTransactionBuilder;

#[cfg(feature = "evm")]
use crate::evm::EVMTransactionBuilder;

#[cfg(feature = "near")]
use crate::near::NearTransactionBuilder;

#[cfg(feature = "solana")]
use crate::solana::SolanaTransactionBuilder;

#[cfg(feature = "starknet")]
use crate::starknet::StarknetTransactionBuilder;

#[cfg(feature = "sui")]
use crate::sui::SuiTransactionBuilder;

#[cfg(feature = "ton")]
use crate::ton::TonTransactionBuilder;

#[cfg(feature = "zcash")]
use crate::zcash::ZcashTransactionBuilder;

#[cfg(feature = "near")]
pub type NEAR = NearTransactionBuilder;

#[cfg(feature = "evm")]
pub type EVM = EVMTransactionBuilder;

#[cfg(feature = "bitcoin")]
pub type BITCOIN = BitcoinTransactionBuilder;

#[cfg(feature = "solana")]
pub type SOLANA = SolanaTransactionBuilder;

#[cfg(feature = "aptos")]
pub type APTOS = AptosTransactionBuilder;

#[cfg(feature = "sui")]
pub type SUI = SuiTransactionBuilder;

#[cfg(feature = "zcash")]
pub type ZCASH = ZcashTransactionBuilder;

#[cfg(feature = "starknet")]
pub type STARKNET = StarknetTransactionBuilder;

#[cfg(feature = "ton")]
pub type TON = TonTransactionBuilder;
