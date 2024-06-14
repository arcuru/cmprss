# cmprss

[![build](https://img.shields.io/github/actions/workflow/status/atuinsh/atuin/rust.yml?style=flat-square)](https://github.com/arcuru/cmprss/actions?query=workflow%3ANix)
[![crates.io](https://img.shields.io/crates/v/cmprss.svg?style=flat-square)](https://crates.io/crates/cmprss)
[![coverage](https://img.shields.io/codecov/c/github/arcuru/cmprss)](https://codecov.io/gh/arcuru/cmprss)
![license](https://img.shields.io/github/license/arcuru/cmprss)

**Status: Alpha.**
CLI is relatively stable but likely contains bugs, and there may be future breaking changes.

A compression multi-tool for the command line.
Replace `tar` with something you can remember.
[Relevant XKCD](https://xkcd.com/1168/).

Currently supports:

- bzip2
- gzip
- tar
- xz

## Install

Installation is available through source code and cargo. `cargo install cmprss` will install the latest version.

For Nix users, the repository contains a flake and an overlay. `nix run github:arcuru/cmprss`

## Usage

The primary goal is to infer behavior based on the input, so that you don't need to remember esoteric CLI arguments.

`cmprss` supports being very explicit about the inputs and outputs for scripting, but will also behave intelligently when you leave out info.

All commands read from left to right, input is always either piped from `stdin` or the first filename(s) specified, and output is either `stdout` or the last filename/directory.

The easiest way to understand is to look at some examples

Compress a file with gzip

```bash
cmprss file.txt file.txt.gz
```

Compress 2 files into a tar archive

```bash
cmprss file1.txt file2.txt archive.tar
```

Compress stdin with xz

```bash
cat file.txt | cmprss file.xz
```

Extract a tar archive to the current directory

```bash
cmprss archive.tar
```

Extract an xz compressed file

```bash
cmprss file.xz file.txt
```

Extract a gzip compressed file to stdout

```bash
cmprss file.txt.gz > file.txt
```

`cmprss` doesn't yet support multiple levels of archiving, like `.tar.gz`, but they are easy to work with using pipes

```bash
cmprss tar uncompressed_dir | cmprss gz > out.tar.gz
cmprss gzip --extract out.tar.gz | cmprss tar -e output_dir

# Or a full roundtrip in one line
cmprss tar dir | cmprss gz | cmprss gz -e | cmprss tar -e
```

### Examples of Explicit Behavior

All these examples will work with _any_ of the supported compression formats, provided that they support the input/output formats.

If output filenames are left out, `cmprss` will try to infer the filename based on the compression type.

Compress a file/directory to a `tar` archive:

```bash
cmprss tar filename # outputs to filename.tar
cmprss tar filename my_preferred_output_name.tar
```

Compress 2 files/directories into a `tar` archive:

```bash
cmprss tar dir_1/ dir_2/ combined.tar
cmprss tar file_1.txt file_2.txt # outputs to file_1.txt.tar
```

Extract a `tar` archive:

```bash
cmprss tar --extract archive.tar # extracts to the current directory
cmprss tar -e archive.tar custom_output_directory
```

`cmprss` will detect if `stdin` or `stdout` is a pipe, and use those for I/O where it makes sense.

Create and extract a `tar.gz` archive with pipes:

```bash
cmprss tar directory | cmprss gzip > directory.tar.gz
cmprss gzip --extract directory.tar.gz | cmprss tar -e new_directory

# Or a full roundtrip in one line
cmprss tar directory_1/ directory_2/ | cmprss gzip | cmprss gzip -e | cmprss tar -e new_directory
```

## Contributing

### Development Environment

The primary supported developer environment is defined in the `flake.nix` file.
This is a [Nix Flake](https://nixos.wiki/wiki/Flakes) that pins versions of all packages used by `cmprss`.
It includes a `devShell` that can be used with [direnv](https://direnv.net/) to use the tools each time you enter the directory.

That being said, `cmprss` is a very standard Rust application and should work with recent Rust toolchains.

The CI runs on both a stable Rust toolchain and the pinned Nix versions to verify correctness of both.

If you run into any issues developing with either the Nix environment or a stable Rust environment, please open a Github issue with the details.

### Conventional Commits

Commits should conform to the [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/) standard.

A script to help create conforming commits is provided in `bin/commit.sh`, or via `task commit`.

### Test Coverage

PRs that improve the test coverage are encouraged.

Test coverage can be measured using `cargo llvm-cov report` and `cargo tarpaulin`.
