# run clippy
lint:
    cargo clippy --all-targets -- -D clippy::all -D clippy::nursery

# run fmt
fmt:
    cargo fmt --check