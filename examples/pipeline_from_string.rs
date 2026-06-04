//! Resolve a dotted format string (e.g. `tar.gz`, `tgz`) into a `Pipeline` and
//! use it to compress a directory.
//!
//! Run with:
//!
//! ```sh
//! cargo run --example pipeline_from_string -- tar.gz some_dir output.tar.gz
//! ```
//!
//! `chain_from_format_str` is the same lookup the CLI uses for codec-only
//! positional invocations. It returns the codecs in
//! innermost → outermost order, ready to feed straight into
//! [`Pipeline::with_format`].

use cmprss::{CmprssInput, CmprssOutput, Compressor, Pipeline, Result, chain_from_format_str};
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

    let chain = chain_from_format_str(&format)
        .unwrap_or_else(|| panic!("unknown format string: {format:?}"));
    let pipeline = Pipeline::with_format(chain, format);

    pipeline.compress(CmprssInput::Path(vec![input]), CmprssOutput::Path(output))
}
