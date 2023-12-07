# list available commands
@list:
    just --list

# run CI locally
@ci:
    act

# run all checks
check: fmt test
    cargo clippy
    pre-commit run --all-files --show-diff-on-failure
    nix flake check

# format everything
@fmt:
    just --fmt --unstable
    cargo fmt --all

# run tests
test:
    cargo test --all
