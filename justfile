# Run linting
lint:
    cargo clippy --all-targets -- -D clippy::all -D clippy::nursery

# Check formatting
fmt:
    cargo fmt --check

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
    cargo test --test '*'