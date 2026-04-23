//! Shared I/O plumbing for single-stream compressors.
//!
//! Every single-file codec (gzip, xz, bzip2, zstd, lz4, brotli, snappy, lzma)
//! has the same shape: resolve the input into a `Read`, resolve the output
//! into a `Write`, reject directory inputs/outputs, and forward the in-memory
//! `Reader`/`Writer` variants untouched for pipeline stages. These helpers
//! consolidate that plumbing so each backend only expresses its codec choice.

use crate::utils::{CmprssInput, CmprssOutput, Result};
use anyhow::bail;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};

/// Resolve a `CmprssInput` into a single boxed `Read` stream for single-stream
/// codecs. Returns the stream together with the input file's size when known
/// (used to drive progress bars).
///
/// Bails when multiple input paths are given, or when a path input is a
/// directory — single-stream codecs operate on exactly one byte stream.
pub fn open_input(input: CmprssInput, name: &str) -> Result<(Box<dyn Read + Send>, Option<u64>)> {
    match input {
        CmprssInput::Path(paths) => {
            if paths.len() > 1 {
                bail!("Multiple input files not supported for {name}");
            }
            let path = &paths[0];
            if path.is_dir() {
                bail!("{name} does not operate on directories; specify a file instead");
            }
            let size = std::fs::metadata(path)?.len();
            let reader: Box<dyn Read + Send> = Box::new(BufReader::new(File::open(path)?));
            Ok((reader, Some(size)))
        }
        CmprssInput::Pipe(stdin) => Ok((Box::new(BufReader::new(stdin)), None)),
        CmprssInput::Reader(reader) => Ok((reader.0, None)),
    }
}

/// Bail if the output path refers to an existing directory. Single-stream
/// codecs always emit a single byte stream, so they can't write to a
/// directory.
pub fn guard_file_output(output: &CmprssOutput, name: &str) -> Result {
    if let CmprssOutput::Path(path) = output
        && path.is_dir()
    {
        bail!("{name} does not operate on directories; specify an output file instead");
    }
    Ok(())
}

/// Open a `CmprssOutput` as a boxed `Write`.
///
/// Callers must destructure `CmprssOutput::Writer` themselves before calling
/// this — the in-memory `Writer` is already a boxed `Write` and doesn't need
/// an additional buffering layer.
pub fn open_output(output: &CmprssOutput) -> Result<Box<dyn Write + Send + '_>> {
    match output {
        CmprssOutput::Path(path) => Ok(Box::new(BufWriter::new(File::create(path)?))),
        CmprssOutput::Pipe(stdout) => Ok(Box::new(BufWriter::new(stdout))),
        CmprssOutput::Writer(_) => {
            unreachable!("open_output called with CmprssOutput::Writer; destructure it first")
        }
    }
}
