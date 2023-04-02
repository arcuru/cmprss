extern crate tar;

use std::fs::File;
use std::path::Path;
use tar::Builder;

/// Compress an input file or directory into a tar archive.
pub fn compress<P: AsRef<Path>>(in_file: &Path, out_file: P) {
    let mut archive = Builder::new(File::create(out_file).unwrap());
    if in_file.is_file() {
        archive
            .append_file(
                in_file.file_name().unwrap(),
                &mut File::open(in_file).unwrap(),
            )
            .unwrap();
    } else if in_file.is_dir() {
        archive
            .append_dir_all(in_file.file_name().unwrap(), in_file)
            .unwrap();
    }
    archive.finish().unwrap();
}
