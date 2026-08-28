/// EIP-1559 transaction type
#[cfg(feature = "evm")]
pub const EIP_1559_TYPE: u8 = 0x02;
/// Length of an Ed25519 public key
#[cfg(any(feature = "near", feature = "solana", feature = "ton"))]
pub const ED25519_PUBLIC_KEY_LENGTH: usize = 32;
/// Length of a secp256k1 public key
#[cfg(feature = "near")]
pub const SECP256K1_PUBLIC_KEY_LENGTH: usize = 64;
/// Length of a secp256k1 signature
#[cfg(feature = "near")]
pub const SECP256K1_SIGNATURE_LENGTH: usize = 65;
/// Size of a single component of an Ed25519 signature.
#[cfg(any(feature = "near", feature = "solana", feature = "ton"))]
pub const COMPONENT_SIZE: usize = 32;
/// Length of an Ed25519 signature
#[cfg(any(feature = "near", feature = "solana", feature = "ton"))]
pub const ED25519_SIGNATURE_LENGTH: usize = COMPONENT_SIZE * 2;
