extern crate tar;

use std::fs::File;
use std::path::Path;
use tar::{Archive, Builder};

/// Return the standard extension for the tar format.
pub fn extension() -> &'static str {
    "tar"
}

/// Compress an input file or directory into a tar archive.
pub fn compress<I: AsRef<Path>, O: AsRef<Path>>(in_file: I, out_file: O) {
    let in_file = in_file.as_ref();
    let out_file = out_file.as_ref();
    println!(
        "tar: Compressing {} into {}",
        in_file.display(),
        out_file.display()
    );
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

/// Extract the archive into the current directory
pub fn extract<I: AsRef<Path>, O: AsRef<Path>>(in_file: I, out_directory: O) {
    let in_file = in_file.as_ref();
    let out_directory = out_directory.as_ref();
    println!(
        "tar: Extracting {} into {}",
        in_file.display(),
        out_directory.display()
    );
    let mut archive = Archive::new(File::open(in_file).unwrap());
    archive.unpack(out_directory).unwrap();
}
