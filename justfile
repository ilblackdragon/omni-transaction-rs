# Run linting
lint:
    cargo clippy --all-targets -- -D clippy::all -D clippy::nursery

# Check formatting
fmt:
    cargo fmt --check

# Check docs (per feature too: a doc link to a cfg-gated item only dangles
# when that item is compiled out, which the all-features build never catches)
doc:
    RUSTDOCFLAGS="-D warnings" cargo doc
    RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --no-default-features --features near
    RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --no-default-features --features evm
    RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --no-default-features --features bitcoin
    RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --no-default-features --features solana
    RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --no-default-features --features aptos
    RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --no-default-features --features sui
    RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --no-default-features --features zcash
    RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --no-default-features --features starknet
    RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --no-default-features --features ton
    
# Verify all compiles
check:
    cargo check

# Verify all compiles with wasm
check-wasm:
    cargo check --target wasm32-unknown-unknown --no-default-features
    cargo check --target wasm32-unknown-unknown
    
# Run unit tests
test-unit:
    cargo test --doc --no-default-features
    cargo test --doc --no-default-features --features bitcoin
    cargo test --doc --no-default-features --features evm
    cargo test --doc --no-default-features --features near
    cargo test --doc --no-default-features --features solana
    cargo test --doc --no-default-features --features aptos
    cargo test --doc --no-default-features --features sui
    cargo test --doc --no-default-features --features zcash
    cargo test --doc --no-default-features --features starknet
    cargo test --doc --no-default-features --features ton
    cargo test --doc
    cargo test --lib --no-default-features
    cargo test --lib --no-default-features --features bitcoin
    cargo test --lib --no-default-features --features evm
    cargo test --lib --no-default-features --features near
    cargo test --lib --no-default-features --features solana
    cargo test --lib --no-default-features --features aptos
    cargo test --lib --no-default-features --features sui
    cargo test --lib --no-default-features --features zcash
    cargo test --lib --no-default-features --features starknet
    cargo test --lib --no-default-features --features ton
    cargo test --lib

# Run integration tests
test-integration:
    RUST_TEST_THREADS=1 cargo test --test '*' --no-default-features
    RUST_TEST_THREADS=1 cargo test --test '*'

# Build the project
build:
    cargo build

# Build the project for wasm
build-wasm:
    cargo build --target wasm32-unknown-unknown --release
