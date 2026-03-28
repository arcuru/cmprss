mod bzip2;
mod gzip;
mod lz4;
mod pipeline;
mod tar;
mod xz;
mod zip;
mod zstd;

pub use bzip2::{Bzip2, Bzip2Args};
pub use gzip::{Gzip, GzipArgs};
pub use lz4::{Lz4, Lz4Args};
pub use pipeline::Pipeline;
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
        _ => None,
    }
}
