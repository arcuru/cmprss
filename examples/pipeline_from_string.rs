//! Resolve a dotted format string (e.g. `tar.gz`, `tgz`) into a `Pipeline` and
//! use it to compress a directory.
//!
//! Run with:
//!
//! ```sh
//! cargo run --example pipeline_from_string -- tar.gz some_dir output.tar.gz
//! ```
//!
//! `Pipeline::from_format_str` mirrors the CLI's codec-only positional
//! inference: hand it a string, get back a ready-to-run pipeline.

use cmprss::{CmprssInput, CmprssOutput, Compressor, Pipeline, Result};
use std::path::PathBuf;

fn main() -> Result {
    let mut args = std::env::args().skip(1);
    let format = args
        .next()
        .expect("usage: pipeline_from_string <format> <input> <output>");
    let input: PathBuf = args
        .next()
        .expect("usage: pipeline_from_string <format> <input> <output>")
        .into();
    let output: PathBuf = args
        .next()
        .expect("usage: pipeline_from_string <format> <input> <output>")
        .into();

    let pipeline = Pipeline::from_format_str(&format)
        .unwrap_or_else(|| panic!("unknown format string: {format:?}"));

    pipeline.compress(CmprssInput::Path(vec![input]), CmprssOutput::Path(output))
}
