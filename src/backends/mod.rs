#[cfg(feature = "brotli")]
mod brotli;
#[cfg(feature = "bzip2")]
mod bzip2;
#[cfg(any(feature = "tar", feature = "zip", feature = "sevenz"))]
mod containers;
#[cfg(feature = "gzip")]
mod gzip;
#[cfg(feature = "lz4")]
mod lz4;
#[cfg(feature = "lzma")]
mod lzma;
mod pipeline;
#[cfg(feature = "sevenz")]
mod sevenz;
#[cfg(feature = "snappy")]
mod snappy;
#[cfg(any(
    feature = "gzip",
    feature = "xz",
    feature = "bzip2",
    feature = "zstd",
    feature = "lz4",
    feature = "brotli",
    feature = "snappy",
    feature = "lzma",
))]
mod stream;
#[cfg(feature = "tar")]
mod tar;
#[cfg(feature = "xz")]
mod xz;
#[cfg(feature = "zip")]
mod zip;
#[cfg(feature = "zstd")]
mod zstd;

#[cfg(feature = "brotli")]
pub use brotli::Brotli;
#[cfg(all(feature = "brotli", feature = "cli"))]
pub use brotli::BrotliArgs;
#[cfg(feature = "bzip2")]
pub use bzip2::Bzip2;
#[cfg(all(feature = "bzip2", feature = "cli"))]
pub use bzip2::Bzip2Args;
#[cfg(feature = "gzip")]
pub use gzip::Gzip;
#[cfg(all(feature = "gzip", feature = "cli"))]
pub use gzip::GzipArgs;
#[cfg(feature = "lz4")]
pub use lz4::Lz4;
#[cfg(all(feature = "lz4", feature = "cli"))]
pub use lz4::Lz4Args;
#[cfg(feature = "lzma")]
pub use lzma::Lzma;
#[cfg(all(feature = "lzma", feature = "cli"))]
pub use lzma::LzmaArgs;
pub use pipeline::Pipeline;
#[cfg(feature = "sevenz")]
pub use sevenz::SevenZ;
#[cfg(all(feature = "sevenz", feature = "cli"))]
pub use sevenz::SevenZArgs;
#[cfg(feature = "snappy")]
pub use snappy::Snappy;
#[cfg(all(feature = "snappy", feature = "cli"))]
pub use snappy::SnappyArgs;
#[cfg(feature = "tar")]
pub use tar::Tar;
#[cfg(all(feature = "tar", feature = "cli"))]
pub use tar::TarArgs;
#[cfg(feature = "xz")]
pub use xz::Xz;
#[cfg(all(feature = "xz", feature = "cli"))]
pub use xz::XzArgs;
#[cfg(feature = "zip")]
pub use zip::Zip;
#[cfg(all(feature = "zip", feature = "cli"))]
pub use zip::ZipArgs;
#[cfg(feature = "zstd")]
pub use zstd::Zstd;
#[cfg(all(feature = "zstd", feature = "cli"))]
pub use zstd::ZstdArgs;

use crate::utils::Compressor;

/// Create a default compressor instance from an extension or name string.
/// This is the single canonical lookup table for all compressor types.
///
/// Arms for codecs whose Cargo feature is disabled simply aren't compiled in,
/// so an unknown-to-this-build codec name returns `None` just like a typo.
pub fn compressor_from_str(s: &str) -> Option<Box<dyn Compressor>> {
    match s {
        #[cfg(feature = "tar")]
        "tar" => Some(Box::<Tar>::default()),
        #[cfg(feature = "gzip")]
        "gzip" | "gz" => Some(Box::<Gzip>::default()),
        #[cfg(feature = "xz")]
        "xz" => Some(Box::<Xz>::default()),
        #[cfg(feature = "bzip2")]
        "bzip2" | "bz2" => Some(Box::<Bzip2>::default()),
        #[cfg(feature = "zip")]
        "zip" => Some(Box::<Zip>::default()),
        #[cfg(feature = "zstd")]
        "zstd" | "zst" => Some(Box::<Zstd>::default()),
        #[cfg(feature = "lz4")]
        "lz4" => Some(Box::<Lz4>::default()),
        #[cfg(feature = "brotli")]
        "brotli" | "br" => Some(Box::<Brotli>::default()),
        #[cfg(feature = "snappy")]
        "snappy" | "sz" => Some(Box::<Snappy>::default()),
        #[cfg(feature = "lzma")]
        "lzma" => Some(Box::<Lzma>::default()),
        #[cfg(feature = "sevenz")]
        "7z" | "sevenz" => Some(Box::<SevenZ>::default()),
        _ => None,
    }
}

/// Resolve an extension to a compressor chain in innermost→outermost order.
/// Single-codec extensions (`gz`, `xz`, `tar`, …) produce a one-element chain;
/// compound shortcut extensions (`tgz`, `tbz`, `tbz2`, `txz`, `tzst`) expand
/// into the chain they represent (e.g. `tgz` → `[tar, gz]`).
///
/// This is the single source of truth for what any archive-like extension
/// means. Both single extensions and compound shortcuts flow through here.
pub fn chain_from_ext(ext: &str) -> Option<Vec<Box<dyn Compressor>>> {
    match ext {
        #[cfg(all(feature = "tar", feature = "gzip"))]
        "tgz" => Some(vec![Box::<Tar>::default(), Box::<Gzip>::default()]),
        #[cfg(all(feature = "tar", feature = "bzip2"))]
        "tbz" | "tbz2" => Some(vec![Box::<Tar>::default(), Box::<Bzip2>::default()]),
        #[cfg(all(feature = "tar", feature = "xz"))]
        "txz" => Some(vec![Box::<Tar>::default(), Box::<Xz>::default()]),
        #[cfg(all(feature = "tar", feature = "zstd"))]
        "tzst" => Some(vec![Box::<Tar>::default(), Box::<Zstd>::default()]),
        _ => compressor_from_str(ext).map(|c| vec![c]),
    }
}

/// Resolve a dotted format string (e.g. `tar.gz`, `tgz`, `xz`) into a
/// compressor chain. Every dot-separated segment is resolved via
/// `chain_from_ext` and concatenated in order. Returns `None` if any
/// segment isn't a known codec or shortcut.
pub fn chain_from_format_str(s: &str) -> Option<Vec<Box<dyn Compressor>>> {
    let mut chain = Vec::new();
    for part in s.split('.') {
        chain.extend(chain_from_ext(part)?);
    }
    if chain.is_empty() { None } else { Some(chain) }
}
