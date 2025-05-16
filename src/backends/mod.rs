mod bzip2;
mod gzip;
mod lz4;
mod multi_level;
mod tar;
mod xz;
mod zip;
mod zstd;

pub use bzip2::{Bzip2, Bzip2Args};
pub use gzip::{Gzip, GzipArgs};
pub use lz4::{Lz4, Lz4Args};
pub use multi_level::MultiLevelCompressor;
pub use tar::{Tar, TarArgs};
pub use xz::{Xz, XzArgs};
pub use zip::{Zip, ZipArgs};
pub use zstd::{Zstd, ZstdArgs};
