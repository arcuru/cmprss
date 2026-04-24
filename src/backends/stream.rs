//! Shared I/O plumbing for single-stream compressors.
//!
//! Every single-file codec (gzip, xz, bzip2, zstd, lz4, brotli, snappy, lzma)
//! has the same shape: resolve the input into a `Read`, resolve the output
//! into a `Write`, reject directory inputs/outputs, and forward the in-memory
//! `Reader`/`Writer` variants untouched for pipeline stages. These helpers
//! consolidate that plumbing so each backend only expresses its codec choice.

use crate::progress::{OutputTarget, ProgressArgs, copy_with_progress};
use crate::utils::{CmprssInput, CmprssOutput, Result, WriteWrapper};
use anyhow::bail;
use std::fs::File;
use std::io::{self, BufReader, BufWriter, Read, Write};

/// Resolve a `CmprssInput` into a single boxed `Read` stream for single-stream
/// codecs. Returns the stream together with the input file's size when known
/// (used to drive progress bars) and a flag indicating whether the input is a
/// pipeline-internal `Reader` (in which case progress should be suppressed in
/// this stage — the innermost stage owns the bar).
///
/// Bails when multiple input paths are given, or when a path input is a
/// directory — single-stream codecs operate on exactly one byte stream.
pub fn open_input(
    input: CmprssInput,
    name: &str,
) -> Result<(Box<dyn Read + Send>, Option<u64>, bool)> {
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
            Ok((reader, Some(size), false))
        }
        CmprssInput::Pipe(stdin) => Ok((Box::new(BufReader::new(stdin)), None, false)),
        CmprssInput::Reader(reader) => Ok((reader.0, None, true)),
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

/// Resolve a `CmprssOutput` into an owned boxed writer plus an `OutputTarget`
/// describing how it should be treated by the copy path (progress bar vs. no
/// progress, etc.). This consumes the output, so callers that still need to
/// inspect the `CmprssOutput` variant should capture what they need before
/// calling.
pub fn prepare_output(output: CmprssOutput) -> Result<(Box<dyn Write + Send>, OutputTarget)> {
    match output {
        CmprssOutput::Writer(WriteWrapper(w)) => Ok((w, OutputTarget::InMemory)),
        CmprssOutput::Pipe(stdout) => Ok((Box::new(BufWriter::new(stdout)), OutputTarget::Stdout)),
        CmprssOutput::Path(path) => Ok((
            Box::new(BufWriter::new(File::create(path)?)),
            OutputTarget::File,
        )),
    }
}

/// Copy bytes from `reader` through `writer`, branching on whether progress
/// is relevant: pipeline-internal stages (either writing to an in-memory pipe
/// or reading from one) use a bare `io::copy` with no progress, while
/// user-facing outputs go through `copy_with_progress` to show a progress bar
/// when configured. `pipeline_inner` is set when the input comes from an
/// upstream pipeline stage — in that case we don't know the total size and
/// the innermost stage already owns the progress bar, so we skip ours.
pub fn copy_stream<R: Read, W: Write>(
    mut reader: R,
    mut writer: W,
    file_size: Option<u64>,
    pipeline_inner: bool,
    progress_args: &ProgressArgs,
    target: OutputTarget,
) -> Result {
    if pipeline_inner || target == OutputTarget::InMemory {
        io::copy(&mut reader, &mut writer)?;
    } else {
        copy_with_progress(
            reader,
            writer,
            progress_args.chunk_size.size_in_bytes,
            file_size,
            progress_args.progress,
            target,
        )?;
    }
    Ok(())
}
