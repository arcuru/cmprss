# cmprss Development Commands
# Run `just` to see available recipes

alias b := build
alias t := test

[private]
default:
    @just --list

# =============================================================================
# Development Workflows
# =============================================================================

# Quick development feedback (build + test + lint)
dev:
    just build
    just test
    just lint clippy

# Run automatic fixes (clippy fix + nix fixes + format)
fix:
    cargo clippy --fix --allow-dirty --allow-staged --all-targets --all-features
    statix fix .
    deadnix --edit .
    just fmt

# =============================================================================
# Building
# =============================================================================

# Build the project (debug or release)
build mode='debug':
    cargo build --all-targets --all-features {{ if mode == "release" { "--release" } else { "" } }} --quiet

# =============================================================================
# Testing
# =============================================================================

# Run tests
test *args:
    #!/usr/bin/env bash
    set -e
    args="{{ args }}"

    if [ -z "$args" ]; then
        cargo nextest run --no-default-features
        exit 0
    fi

    case "$args" in
        full)
            ./bin/test.sh
            ;;
        *)
            cargo nextest run --no-default-features "$args"
            ;;
    esac

# =============================================================================
# Linting (Static Analysis)
# =============================================================================

# Run linter(s): clippy, deny, typos, statix, deadnix, shellcheck, actionlint, all
lint +tools='clippy deny typos statix deadnix shellcheck actionlint':
    #!/usr/bin/env bash
    set -e
    for tool in {{ tools }}; do
        case "$tool" in
            clippy)
                echo "=== Running clippy ==="
                cargo clippy --all-targets --all-features -- -D warnings
                ;;
            deny)
                echo "=== Running cargo-deny ==="
                cargo deny check
                ;;
            typos)
                echo "=== Running typos ==="
                typos --config .config/typos.toml
                ;;
            statix)
                echo "=== Running statix ==="
                statix check .
                ;;
            deadnix)
                echo "=== Running deadnix ==="
                deadnix --fail .
                ;;
            shellcheck)
                echo "=== Running shellcheck ==="
                find . -name "*.sh" -type f -exec shellcheck {} +
                ;;
            actionlint)
                echo "=== Running actionlint ==="
                find .github/workflows -name "*.yml" -exec actionlint {} +
                ;;
            all)
                just lint clippy deny typos statix deadnix shellcheck actionlint
                ;;
            *)
                echo "Unknown linter: $tool"
                echo "Options: clippy, deny, typos, statix, deadnix, shellcheck, actionlint, all"
                exit 1
                ;;
        esac
    done

# =============================================================================
# Formatting
# =============================================================================

# Run formatters: (default), check
fmt mode='':
    #!/usr/bin/env bash
    set -e
    case "{{ mode }}" in
        check)
            cargo fmt -- --check
            alejandra . --check --quiet
            prettier --check . --log-level warn
            typos --config .config/typos.toml
            ;;
        *)
            cargo fmt
            alejandra . --quiet
            prettier --write . --log-level warn
            typos --write-changes --config .config/typos.toml
            ;;
    esac

# =============================================================================
# Coverage
# =============================================================================

# Generate coverage report
coverage:
    cargo tarpaulin --skip-clean --include-tests --output-dir coverage --out lcov --no-default-features

# Generate coverage using nix
nix-coverage:
    nix build .#coverage --out-link coverage

# =============================================================================
# CI
# =============================================================================

# Run CI locally: local (default), full (containers), nix
ci mode='local':
    #!/usr/bin/env bash
    set -e
    case "{{ mode }}" in
        local)
            just fix
            just lint
            just build
            just test
            just build release
            ;;
        full)
            act
            ;;
        nix)
            just nix check
            ;;
        *)
            echo "Unknown mode: {{ mode }}"
            echo "Options: local, full, nix"
            exit 1
            ;;
    esac

# =============================================================================
# Nix
# =============================================================================

# Nix commands: build, check, fmt
nix action='check':
    #!/usr/bin/env bash
    set -e
    case "{{ action }}" in
        build)
            nix build
            ;;
        check)
            nix flake check
            ;;
        fmt)
            nix fmt
            ;;
        *)
            echo "Unknown action: {{ action }}"
            echo "Options: build, check, fmt"
            exit 1
            ;;
    esac

# =============================================================================
# Commit
# =============================================================================

# Interactive conventional commit
commit:
    ./bin/commit.sh
