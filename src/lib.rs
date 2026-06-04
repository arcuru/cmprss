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

pub mod backends;
pub mod job;
pub mod progress;
#[cfg(test)]
pub mod test_utils;
pub mod utils;

pub use backends::{
    Brotli, BrotliArgs, Bzip2, Bzip2Args, Gzip, GzipArgs, Lz4, Lz4Args, Lzma, LzmaArgs, Pipeline,
    SevenZ, SevenZArgs, Snappy, SnappyArgs, Tar, TarArgs, Xz, XzArgs, Zip, ZipArgs, Zstd, ZstdArgs,
    chain_from_ext, chain_from_format_str, compressor_from_str,
};
pub use utils::{
    CmprssInput, CmprssOutput, CommonArgs, CompressionLevel, CompressionLevelValidator, Compressor,
    DefaultCompressionValidator, ExtractedTarget, LevelArgs, PassthroughWriter, ReadWrapper,
    Result, StreamCodec, StreamWriter, WriteWrapper,
};
