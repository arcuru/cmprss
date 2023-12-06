# list available commands
@list:
    just --list

# run CI locally
@ci:
    act

# run all checks
check:
    just fmt
    pre-commit run --all-files --show-diff-on-failure
    nix flake check

# format everything
@fmt:
    just --fmt --unstable
    cargo fmt --all
