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
    cargo check --target wasm32-unknown-unknown
    
# Run unit tests
test-unit:
    cargo test --lib

# Run integration tests
test-integration:
    RUST_TEST_THREADS=1 cargo test --test '*'

# Build the project
build:
    cargo build

# Build the project for wasm
build-wasm:
    cargo build --target wasm32-unknown-unknown --release