//! cmprss as a library.
//!
//! The `cmprss` binary is a thin clap wrapper over this crate; everything
//! interesting — the codecs, the streaming traits, the pipeline composer,
//! the format-string parser — lives here and is usable from other Rust code.
//!
//! Two trait surfaces matter to most users:
//!
//! * [`Compressor`] is the high-level "compress this input into this output"
//!   interface implemented by every codec ([`Gzip`], [`Xz`], [`Zstd`], …)
//!   and by [`Pipeline`] for compound formats like `.tar.gz`.
//! * [`StreamCodec`] is the lower-level decorator interface. Single-stream
//!   codecs (gzip, xz, bzip2, zstd, lz4, brotli, snappy, lzma) implement it
//!   so they can be stacked as `Write`/`Read` wrappers on a single thread —
//!   this is what `Pipeline` uses to compose stages without spawning workers.
//!
//! For format-string parsing (e.g. turning `"tar.gz"` or `"tgz"` into a
//! ready-to-run [`Pipeline`]) see [`chain_from_format_str`] and
//! [`chain_from_ext`].
//!
//! # Quick start
//!
//! Single-codec compression: construct a codec with `Default::default()`
//! (overriding any public fields you care about) and call
//! [`Compressor::compress`] with a `CmprssInput` and `CmprssOutput`:
//!
//! ```no_run
//! # #[cfg(feature = "gzip")]
//! # fn run() -> cmprss::Result {
//! use cmprss::{CmprssInput, CmprssOutput, Compressor, Gzip};
//!
//! let gz = Gzip {
//!     compression_level: 9,
//!     ..Gzip::default()
//! };
//! gz.compress(
//!     CmprssInput::Path(vec!["input.txt".into()]),
//!     CmprssOutput::Path("input.txt.gz".into()),
//! )?;
//! # Ok(()) }
//! ```
//!
//! Compound formats: stack codecs into a [`Pipeline`] in innermost →
//! outermost order. For `tar.gz` that's `[Tar, Gzip]` — tar produces the
//! archive bytes, gzip wraps them.
//!
//! ```no_run
//! # #[cfg(all(feature = "tar", feature = "gzip"))]
//! # fn run() -> cmprss::Result {
//! use cmprss::{CmprssInput, CmprssOutput, Compressor, Gzip, Pipeline, Tar};
//!
//! let pipeline = Pipeline::new(vec![
//!     Box::new(Tar::default()),
//!     Box::new(Gzip::default()),
//! ]);
//! pipeline.compress(
//!     CmprssInput::Path(vec!["my_dir".into()]),
//!     CmprssOutput::Path("my_dir.tar.gz".into()),
//! )?;
//! # Ok(()) }
//! ```
//!
//! Or let [`Pipeline::from_format_str`] turn a dotted string into the same
//! chain (this is the same lookup the CLI uses for codec-only positional
//! invocations):
//!
//! ```no_run
//! # #[cfg(all(feature = "tar", feature = "gzip"))]
//! # fn run() -> cmprss::Result {
//! use cmprss::{CmprssInput, CmprssOutput, Compressor, Pipeline};
//!
//! let pipeline = Pipeline::from_format_str("tgz").expect("known format");
//! pipeline.compress(
//!     CmprssInput::Path(vec!["my_dir".into()]),
//!     CmprssOutput::Path("my_dir.tgz".into()),
//! )?;
//! # Ok(()) }
//! ```
//!
//! Worked examples for each of these patterns live under the crate's
//! `examples/` directory and run via `cargo run --example <name>`.
//!
//! # Features
//!
//! Each codec is gated behind a Cargo feature of the same name (`gzip`, `xz`,
//! `bzip2`, `zstd`, `lz4`, `brotli`, `snappy`, `lzma`, `tar`, `zip`,
//! `sevenz`). The aggregate `full` feature enables them all and is the
//! default. Disable default features and opt back in to a subset to shrink
//! the dependency tree:
//!
//! ```toml
//! cmprss = { version = "0.4", default-features = false, features = ["gzip", "tar"] }
//! ```
//!
//! The CLI surface (clap-derived `XArgs` structs, `X::new(args)`
//! constructors, `CommonArgs`, `LevelArgs`, and the `job` dispatch module)
//! lives behind a `cli` feature that is also on by default. Library callers
//! who don't want clap, clap_complete, or clap_mangen in their dep tree can
//! opt out of `cli` and construct codecs via `Default::default()` plus the
//! public fields on each codec struct.

pub mod backends;
#[cfg(feature = "cli")]
pub mod job;
pub mod progress;
#[cfg(test)]
pub mod test_utils;
pub mod utils;

pub use backends::{Pipeline, chain_from_ext, chain_from_format_str, compressor_from_str};

#[cfg(feature = "brotli")]
pub use backends::Brotli;
#[cfg(all(feature = "brotli", feature = "cli"))]
pub use backends::BrotliArgs;
#[cfg(feature = "bzip2")]
pub use backends::Bzip2;
#[cfg(all(feature = "bzip2", feature = "cli"))]
pub use backends::Bzip2Args;
#[cfg(feature = "gzip")]
pub use backends::Gzip;
#[cfg(all(feature = "gzip", feature = "cli"))]
pub use backends::GzipArgs;
#[cfg(feature = "lz4")]
pub use backends::Lz4;
#[cfg(all(feature = "lz4", feature = "cli"))]
pub use backends::Lz4Args;
#[cfg(feature = "lzma")]
pub use backends::Lzma;
#[cfg(all(feature = "lzma", feature = "cli"))]
pub use backends::LzmaArgs;
#[cfg(feature = "sevenz")]
pub use backends::SevenZ;
#[cfg(all(feature = "sevenz", feature = "cli"))]
pub use backends::SevenZArgs;
#[cfg(feature = "snappy")]
pub use backends::Snappy;
#[cfg(all(feature = "snappy", feature = "cli"))]
pub use backends::SnappyArgs;
#[cfg(feature = "tar")]
pub use backends::Tar;
#[cfg(all(feature = "tar", feature = "cli"))]
pub use backends::TarArgs;
#[cfg(feature = "xz")]
pub use backends::Xz;
#[cfg(all(feature = "xz", feature = "cli"))]
pub use backends::XzArgs;
#[cfg(feature = "zip")]
pub use backends::Zip;
#[cfg(all(feature = "zip", feature = "cli"))]
pub use backends::ZipArgs;
#[cfg(feature = "zstd")]
pub use backends::Zstd;
#[cfg(all(feature = "zstd", feature = "cli"))]
pub use backends::ZstdArgs;

pub use utils::{
    CmprssInput, CmprssOutput, CompressionLevel, CompressionLevelValidator, Compressor,
    DefaultCompressionValidator, ExtractedTarget, PassthroughWriter, ReadWrapper, Result,
    StreamCodec, StreamWriter, WriteWrapper,
};
#[cfg(feature = "cli")]
pub use utils::{CommonArgs, LevelArgs};
