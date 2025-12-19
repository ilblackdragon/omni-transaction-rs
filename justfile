# Run linting
lint:
    cargo clippy --all-targets -- -D clippy::all -D clippy::nursery

# Check formatting
fmt:
    cargo fmt --check

# Check docs
doc:
    RUSTDOCFLAGS="-D warnings" cargo doc
    
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
    cargo test --doc
    cargo test --lib --no-default-features
    cargo test --lib --no-default-features --features bitcoin
    cargo test --lib --no-default-features --features evm
    cargo test --lib --no-default-features --features near
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
