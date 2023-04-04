extern crate tar;

use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use tar::{Archive, Builder};

/// The standard extension for the tar format.
pub const EXT: &str = "tar";

/// Compress an input file or directory into a tar archive.
pub fn compress_file<I: AsRef<Path>, O: Write>(in_file: I, output: O) {
    let in_file = in_file.as_ref();
    println!("tar: Compressing {}", in_file.display());
    let mut archive = Builder::new(output); //File::create(out_file).unwrap());
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

/// Extract the archive file into a directory
pub fn extract_file<I: AsRef<Path>, O: AsRef<Path>>(input_file: I, out_directory: O) {
    let input_file = input_file.as_ref();
    let out_directory = out_directory.as_ref();
    println!(
        "tar: Extracting {} into {}",
        input_file.display(),
        out_directory.display()
    );
    extract(File::open(input_file).unwrap(), out_directory);
}

/// Extract the archive into a directory
pub fn extract<I: Read, O: AsRef<Path>>(input: I, out_directory: O) {
    let mut archive = Archive::new(input);
    archive.unpack(out_directory.as_ref()).unwrap();
}
