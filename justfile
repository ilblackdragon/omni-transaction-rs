# Run clippy
lint:
    cargo clippy --all-targets -- -D clippy::all -D clippy::nursery

# Run fmt
fmt:
    cargo fmt --check

# Run cargo check
check:
    cargo check

# Run tests
test:
    NEAR_RPC_TIMEOUT_SECS=100 cargo test
