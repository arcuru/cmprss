# Changelog

All notable changes to this project will be documented in this file.

## [0.4.0] - 2026-04-25

### Bug Fixes

- Trigger publish workflow explicitly from release-plz
- Preserve per-stage config via Compressor::clone_boxed
- Follow symlinks in container pre-walk to match walker
- Enable ZIP64 to support archives with entries >4GiB
- Avoid panics on non-UTF8 filenames
- Show known total on pipeline compression bar

### Documentation

- List brotli, snappy, and lzma in supported formats

### Features

- Add brotli compression support
- Add snappy framed compression support
- Add legacy LZMA1 compression support
- Recognize .tgz/.tbz/.tbz2/.txz/.tzst shortcut extensions
- Add --force / -f to overwrite existing output
- Generate shell completions and man page
- Add --list / -l to print archive contents
- Add 7zip backend with sevenz-rust2
- Add --level compression level support
- Add --level compression level support
- Add progress bars with shared pre-walk helper
- Add progress bars during compression
- Add progress bars during extraction
- Add progress bars during compression
- Add progress bars during extraction
- Install shell completions and document usage
- Accept compound format prefix like `tar.gz` as first arg
- Preserve shortcut format string in default output name
- Add --append flag for growing tar and zip archives in place

### Miscellaneous Tasks

- Declare MSRV of 1.88 in Cargo.toml
- Validate aarch64 in nix.yml and attach static binaries to releases
- Pin all GitHub Actions to commit SHAs

### Refactor

- Extract shared I/O helpers for single-stream codecs
- Extract job inference into its own module
- Add LevelArgs::resolve to trim backend constructors
- Extract mechanical helpers from get_job
- Linearize output/action/compressor resolution in get_job
- Eliminate Action::Unknown in favor of Option<Action>
- Gate behind cfg(test) to exclude from release binary
- Unify single + shortcut extension lookup in chain_from_ext
- Collect compressors directly instead of round-tripping names
- Replace wildcard utils imports with explicit ones
- Drop redundant overrides and dead-code allows
- Consolidate stream-codec scaffolding via prepare_output/copy_stream
- Share threaded stage driver across compress/extract/list
- Use inc-based ProgressReader for shared bars
- Move clone_boxed to blanket helper trait
- Collapse --decompress into --extract alias
- Replace non-test unwraps with proper error handling

### Styling

- Rename ExtractedTarget variants to CamelCase
- Unify error message prefixes across job and backends
- Apply cargo fmt

### Testing

- Add lzma, brotli, tar, zip, and tar.* pipeline interop tests
- Add tar.{bz2,zst,lzma,br,lz4,sz} roundtrip tests
- Add snappy and tar.{lzma,br,lz4,sz} interop tests



## [0.3.0] - 2026-03-29

### Bug Fixes

- Improve the compression level helpers
- Disable progress bar in test.sh
- Replace panics with proper Writer handling in all backends
- Use correct changelog_config field in release-plz config
- Correct snapcore/action-publish SHA pin
- Match release-plz branch name prefix in PR detection
- Fetch full git history for release-plz changelog generation

### Documentation

- Add installation instructions
- Update README with badges
- Note that compression libraries are statically compiled

### Features

- Adding support for unencrypted zip files
- Improve tar with pipe support and more
- Add progress bar to gzip
- Add zstd support
- Add lz4 support
- Add static build for bzip2
- Add multi-level compression support

### Miscellaneous Tasks

- Add code coverage uploading
- Improving code coverage infrastructure
- Fixing code coverage upload
- Fixing typo in taskfile
- Overhaul of flake to use flake-parts
- Loosen cargo dep restrictions
- Update nix deps
- Taskfile cleanup
- Remove unnecessary nixpkgs from flake-parts
- Iterate on the flake
- Add vscode files to gitignore
- Iterate on the taskfile
- Stop building tests in nix build .#cmprss
- Fiddle with the nix build and ci
- Pin versions of actions helpers
- Add FUNDING.yml
- Setting up Github<->Codeberg syncing
- Add task clippy:fix
- Run all Tasks even if no files have changed
- Bump nix flake deps
- Bump cargo deps
- Taskfile fixups
- Fix treefmt by pointing directly to a rustfmt binary
- Adding pkg-config for updated cargo deps
- Remove unused KNOWN_EXTENSIONS, replace unreachable fallbacks with asserts
- Statically link xz/lzma, update README
- Bump nix flake deps
- Bump cargo deps
- Add release profile optimizations and dev dep opt-level
- Replace cargo-audit with cargo-deny for comprehensive dependency checks
- Add typos and shfmt to treefmt
- Add shellcheck, actionlint, statix, deadnix as nix checks
- Switch from Taskfile (go-task) to justfile
- Relicense from MIT to AGPL-3.0-or-later
- Add fully static musl build via nix
- Add snapcraft packaging and CI
- Add missing metadata fields to snap
- Migrate release flow to release-plz with publish workflow
- Add git-cliff config for release-plz changelog generation

### Refactor

- Consolidate compression backends into dedicated module
- Consolidate compressor lookup into single registry
- Unify single-level and multi-level compressor selection
- Rename MultiLevelCompressor to Pipeline and update module/comments
- Use io::Error::other() and eliminate unwrap() calls
- Migrate error handling from io::Error to anyhow

### Styling

- Pin rustfmt edition to 2024, simplify treefmt rustfmt config

### Testing

- Move comparison tests into Rust integration suite
- Move the tar comparison tests into Rust
- Add unit tests for pipeline extension scanning and trait methods

## [0.2.0] - 2024-02-27

### Bug Fixes

- Remove leftover comment
- Flush xz encoder/decoder to show correct output size
- Error correction for 0 sized chunks
- Restrict compression levels of bzip2

### Documentation

- Add categories to Cargo.toml
- Update the crate description

### Features

- Add bzip2 support
- Add shortcut for cargo audit
- Switch out just for go-task
- Pretty print the error messages
- Add a progress bar to xz
- Add argument to control progress bar
- Add an option to set the chunk size for monitoring progress
- Allow compression levels 'none' 'fast' 'best'
- Add progress bar to bzip2
- Allow `decompress` as an alias of `extract`
- Add the magic cli

### Miscellaneous Tasks

- Overcomplicate the taskfile
- Add git-cliff to generate changelogs
- Release v0.2.0

### Refactor

- Minor changes to the utils
- Move the args and parsing logic into compressor modules

### Styling

- Fix clippy warnings

### Testing

- Add tests for input parsing
- Add a test script against the official tools

### Build

- Add a commit helper script

## [0.1.0] - 2023-12-09

### Bug Fixes

- Set name for devshell
- Remove unused gzip::EXT
- Removing unnecessary comments
- Fixing bug while checking input extension
- Cleaning up some comments and naming
- Improve error message when inferring output name
- Improve output identification
- Use first file's name to generate output filename
- Default extracted filename for gzip

### Documentation

- Update package description
- Add shorthand alias for gzip
- Add usage examples
- Cleanup the README
- Add contributing guide
- Add note about test coverage
- Renaming my GH account to arcuru
- Update description of gzip roundtrip tests
- Cleanup of the README

### Features

- Adding tar extraction
- Use subcommands
- Make tar operate on generic Readers/Writers
- Adding gzip support
- Allow compressing/extracting to a pipe
- Make the input filename optional
- Allow multiple input files to compression
- Adding test stubs
- Init Rust workflow
- Rewrite cli parsing
- Add justfile for short scripts
- Add flags to ignore just stdin or stdout pipes
- Add test target to justfile
- Add xz support
- Extend the check script
- Add .envrc

### Miscellaneous Tasks

- Bump flake deps
- Expand Rust action
- Ignore direnv cache
- Add act and prettier to the environment
- Add Nix action
- Switch flake to use flake-parts and nci
- All deps
- Nix deps
- Rust deps
- Release v0.1.0

### Refactor

- Redo the dispatching with generics
- Use common arguments struct
- Standardize the interface for compressors
- Use common fn for constructing errors
- Simplifying the compressor API
- Simplify gzip compression and extraction
- Remove the unused extract command

### Styling

- Use descriptive names in CI

### Testing

- Add some tar cli tests with target inference
- Use nextest to run tests

### Build

- Rewrite flake to use crane
- Add separate cmprss package/app
- Add overlay for easier consumption
- Fix overlay output
- Remove code coverage from nix flake check

### Dev

- Switch to using treefmt

## [0.0.1] - 2023-04-02

### Bug Fixes

- Disable clippy pre-commit

### Documentation

- Adding basic cargo info

### Features

- Add initial hello world
- Adding flake for dependency management
- Add tar compression

### Miscellaneous Tasks

- Add nix result folder to gitignore
- Bump flake deps

<!-- generated by git-cliff -->
