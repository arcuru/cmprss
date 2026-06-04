//! Compress a single file with gzip.
//!
//! Run with:
//!
//! ```sh
//! cargo run --example compress_file -- input.txt output.txt.gz
//! ```
//!
//! Demonstrates the minimum lib-side API: construct a codec with
//! `Default::default()` (or set its public fields directly), and call
//! `compress` with a `CmprssInput::Path` and `CmprssOutput::Path`.

use cmprss::{CmprssInput, CmprssOutput, Compressor, Gzip, Result};
use std::path::PathBuf;

fn main() -> Result {
    let mut args = std::env::args().skip(1);
    let input: PathBuf = args
        .next()
        .expect("usage: compress_file <input> <output>")
        .into();
    let output: PathBuf = args
        .next()
        .expect("usage: compress_file <input> <output>")
        .into();

    let gz = Gzip {
        compression_level: 9,
        ..Gzip::default()
    };
    gz.compress(CmprssInput::Path(vec![input]), CmprssOutput::Path(output))
}
