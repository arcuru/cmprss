mod brotli;
mod bzip2;
mod gzip;
mod lz4;
mod lzma;
mod pipeline;
mod snappy;
mod stream;
mod tar;
mod xz;
mod zip;
mod zstd;

pub use brotli::{Brotli, BrotliArgs};
pub use bzip2::{Bzip2, Bzip2Args};
pub use gzip::{Gzip, GzipArgs};
pub use lz4::{Lz4, Lz4Args};
pub use lzma::{Lzma, LzmaArgs};
pub use pipeline::Pipeline;
pub use snappy::{Snappy, SnappyArgs};
pub use tar::{Tar, TarArgs};
pub use xz::{Xz, XzArgs};
pub use zip::{Zip, ZipArgs};
pub use zstd::{Zstd, ZstdArgs};

use crate::utils::Compressor;

/// Create a default compressor instance from an extension or name string.
/// This is the single canonical lookup table for all compressor types.
pub fn compressor_from_str(s: &str) -> Option<Box<dyn Compressor>> {
    match s {
        "tar" => Some(Box::<Tar>::default()),
        "gzip" | "gz" => Some(Box::<Gzip>::default()),
        "xz" => Some(Box::<Xz>::default()),
        "bzip2" | "bz2" => Some(Box::<Bzip2>::default()),
        "zip" => Some(Box::<Zip>::default()),
        "zstd" | "zst" => Some(Box::<Zstd>::default()),
        "lz4" => Some(Box::<Lz4>::default()),
        "brotli" | "br" => Some(Box::<Brotli>::default()),
        "snappy" | "sz" => Some(Box::<Snappy>::default()),
        "lzma" => Some(Box::<Lzma>::default()),
        _ => None,
    }
}
