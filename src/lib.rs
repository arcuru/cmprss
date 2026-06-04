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
//! # Features
//!
//! Each codec is gated behind a Cargo feature of the same name (`gzip`, `xz`,
//! `bzip2`, `zstd`, `lz4`, `brotli`, `snappy`, `lzma`, `tar`, `zip`,
//! `sevenz`). The aggregate `full` feature enables them all; that, plus
//! `interop`, is the default. Disable default features and opt back in to a
//! subset to shrink the dependency tree:
//!
//! ```toml
//! cmprss = { version = "0.4", default-features = false, features = ["gzip", "tar"] }
//! ```

pub mod backends;
pub mod job;
pub mod progress;
#[cfg(test)]
pub mod test_utils;
pub mod utils;

pub use backends::{Pipeline, chain_from_ext, chain_from_format_str, compressor_from_str};

#[cfg(feature = "brotli")]
pub use backends::{Brotli, BrotliArgs};
#[cfg(feature = "bzip2")]
pub use backends::{Bzip2, Bzip2Args};
#[cfg(feature = "gzip")]
pub use backends::{Gzip, GzipArgs};
#[cfg(feature = "lz4")]
pub use backends::{Lz4, Lz4Args};
#[cfg(feature = "lzma")]
pub use backends::{Lzma, LzmaArgs};
#[cfg(feature = "sevenz")]
pub use backends::{SevenZ, SevenZArgs};
#[cfg(feature = "snappy")]
pub use backends::{Snappy, SnappyArgs};
#[cfg(feature = "tar")]
pub use backends::{Tar, TarArgs};
#[cfg(feature = "xz")]
pub use backends::{Xz, XzArgs};
#[cfg(feature = "zip")]
pub use backends::{Zip, ZipArgs};
#[cfg(feature = "zstd")]
pub use backends::{Zstd, ZstdArgs};

pub use utils::{
    CmprssInput, CmprssOutput, CommonArgs, CompressionLevel, CompressionLevelValidator, Compressor,
    DefaultCompressionValidator, ExtractedTarget, LevelArgs, PassthroughWriter, ReadWrapper,
    Result, StreamCodec, StreamWriter, WriteWrapper,
};
