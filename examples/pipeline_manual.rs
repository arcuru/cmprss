//! Build a `tar.gz` pipeline by hand, without going through the format-string
//! parser, and tar+gzip an input directory into an archive.
//!
//! Run with:
//!
//! ```sh
//! cargo run --example pipeline_manual -- some_dir output.tar.gz
//! ```
//!
//! Useful when the lib consumer already knows the chain at compile time and
//! wants to customize codec-specific fields (compression level, etc.)
//! directly rather than relying on per-codec `Default`s.

use cmprss::{CmprssInput, CmprssOutput, Compressor, Gzip, Pipeline, Result, Tar};
use std::path::PathBuf;

fn main() -> Result {
    let mut args = std::env::args().skip(1);
    let input: PathBuf = args
        .next()
        .expect("usage: pipeline_manual <input_dir> <output.tar.gz>")
        .into();
    let output: PathBuf = args
        .next()
        .expect("usage: pipeline_manual <input_dir> <output.tar.gz>")
        .into();

    let gz = Gzip {
        compression_level: 6,
        ..Gzip::default()
    };
    let pipeline = Pipeline::new(vec![Box::new(Tar::default()), Box::new(gz)]);

    pipeline.compress(CmprssInput::Path(vec![input]), CmprssOutput::Path(output))
}
