# cmprss

**Status: Alpha.**
CLI is stable but likely contains bugs, and may have breaking changes.

A compression multi-tool for the CLI.
Replace `tar` with something you can remember.
[Relevant XKCD](https://xkcd.com/1168/).

## Usage

The primary goal of the CLI is to make it easy and consistent to work with any compression format.
All of the examples will work with _any_ of the supported compression formats.
Some formats will fail in certain scenarios, as not all compression formats support all types of input/output; for example `tar` is unable to support compressing from `stdin` and extracting to `stdout`, because it expects to operate on files.

All commands read from left to right, input is always either piped from `stdin` or the first filename(s) specified, and output is either `stdout` or the last filename/directory.

If output filenames are left out, `cmprss` will try to infer the filename based on the compression type.

### Examples

Compress a file/directory to a `tar` archive:

```bash
cmprss tar filename # outputs to archive.tar
cmprss tar filename my_preferred_output_name.tar
```

Compress 2 files/directories into a `tar` archive:

```bash
cmprss tar dir_1/ dir_2/ combined.tar
cmprss tar file_1.txt file_2.txt # outputs to archive.tar
```

Extract a `tar` archive:

```bash
cmprss tar --extract archive.tar # extracts to the current directory
cmprss tar -e archive.tar custom_output_directory
```

`cmprss` will detect if `stdin` or `stdout` is a pipe, and use those for I/O where it makes sense.

Create and extract a `tar.gz` archive with pipes:

```bash
cmprss tar directory_name | cmprss gzip > directory.tar.gz
cmprss gzip --extract directory.tar.gz | cmprss tar --extract new_directory

# Or a full roundtrip in one line
cmprss tar directory_1/ directory_2/ | cmprss gzip | cmprss gzip -e | cmprss tar -e new_directory
```

## Supported formats

- gzip
- tar

# Contributing

## Development Environment

The primary supported developer environment is defined in the `flake.nix` file.
This is a [Nix Flake](https://nixos.wiki/wiki/Flakes) that pins versions of all packages used by `cmprss`.
It includes a `devShell` that can be used with [direnv](https://direnv.net/) to use the tools each time you enter the directory.

That being said, `cmprss` is a very standard Rust application and should work with recent Rust toolchains.

The CI runs on both a stable Rust toolchain and the pinned Nix versions to verify correctness of both.

If you run into any issues developing with either the Nix environment or a stable Rust environment, please open a Github issue with the details.

## Conventional Commits

Commits should conform to the [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/) standard.

## Test Coverage

PRs that improve the test coverage are encouraged.

Test coverage can be measured using `cargo llvm-cov report` and `cargo tarpaulin`.

## @arcuru

I am the only developer on this right now, and I usually develop by committing directly to the `main` branch.
For larger features I _may_ go through a PR to run CI and to have some more easily discoverable documentation of a specific feature.

I will stop commiting directly to `main` as soon as someone else submits a non-trivial PR and then submits a request to remove this section of the README.
