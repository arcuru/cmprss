extern crate flate2;

use flate2::write::GzEncoder;
use flate2::{read::GzDecoder, Compression};
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

/// The standard extension for the gzip format.
pub const EXT: &str = "gz";

pub fn compress_file<I: AsRef<Path>, O: Write>(in_file: I, output: O, level: u32) {
    compress(File::open(in_file).unwrap(), output, level);
}

/// Compress an input file or directory into a gzip archive.
pub fn compress<I: Read, O: Write>(mut input: I, output: O, level: u32) {
    let mut encoder = GzEncoder::new(output, Compression::new(level));

    std::io::copy(&mut input, &mut encoder).unwrap();
    encoder.finish().unwrap();
}

/// Extract a gzipped file
pub fn extract_file<I: AsRef<Path>, O: Write>(in_file: I, output: O) {
    extract(File::open(in_file).unwrap(), output);
}

/// Extract the gzip compressed data
pub fn extract<I: Read, O: Write>(input: I, mut output: O) {
    let mut decoder = GzDecoder::new(input);
    std::io::copy(&mut decoder, &mut output).unwrap();
}
