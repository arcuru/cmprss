# cmprss

**Status: Pre-alpha.**
Lacks formal testing.
CLI is reasonably stable but still being tweaked.

A compression multi-tool for the CLI. Replace `tar` with something you can remember. [Relevant XKCD](https://xkcd.com/1168/).

## Usage

The primary goal of the CLI is to make it easy and consistent to work with any compression format.
All of the examples will work with _any_ of the supported compression formats.
Some formats will fail in certain scenarios, as they don't support certain types of input/output; for example `tar` is unable to support compressing from `stdin` and extracting to `stdout`.

All commands read from left to right, input is always either piped from `stdin` or the first filename specified, and output is either `stdout` or the next filename (the first if using `stdin`, the second if using a filename for input).

If output filenames are left out, `cmprss` will try to infer the filename based on the compression type.

### Examples

Compress a file/directory to a `tar` archive:

```
cmprss tar filename
cmprss tar filename my_preferred_output_name.tar
```

Extract a `tar` archive:

```
cmprss tar --extract filename.tar
cmprss tar --extract filename.tar custom_output_directory
```

`cmprss` will detect if `stdin` or `stdout` is a pipe, and use those for I/O.

Create and extract a `tar.gz` archive with pipes:

```
cmprss tar directory_name | cmprss gzip > directory.tar.gz
cmprss gzip --extract directory.tar.gz | cmprss tar --extract new_directory
```

## Supported formats

- gzip
- tar

TODO: Add more compression algos now that the internal API is mostly stable.
