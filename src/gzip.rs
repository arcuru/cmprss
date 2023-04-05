use crate::utils::*;
use flate2::write::GzEncoder;
use flate2::{read::GzDecoder, Compression};
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

/// The standard extension for the gzip format.
pub const EXT: &str = "gz";

pub struct Gzip {
    pub compression_level: u32,
    pub common_args: CmprssCommonArgs,
}

impl CmprssArgTrait for Gzip {
    fn common_args(&self) -> &CmprssCommonArgs {
        &self.common_args
    }
}

impl CmprssInfo for Gzip {
    /// The standard extension for the gzip format.
    fn extension(&self) -> &str {
        "gz"
    }

    /// Full name for gzip.
    fn name(&self) -> &str {
        "gzip"
    }
}

impl CmprssFile for Gzip {
    /// Compress an input file to gzip
    fn compress_file<I: AsRef<Path>, O: Write>(&self, in_file: I, output: O) {
        self.compress(File::open(in_file).unwrap(), output);
    }

    /// Extract a gzipped file
    fn extract_file<I: AsRef<Path>, O: Write>(&self, in_file: I, output: O) {
        self.extract(File::open(in_file).unwrap(), output);
    }
}

impl CmprssRead for Gzip {
    /// Compress an input file or directory into a gzip archive.
    fn compress<I: Read, O: Write>(&self, mut input: I, output: O) {
        let mut encoder = GzEncoder::new(output, Compression::new(self.compression_level));

        std::io::copy(&mut input, &mut encoder).unwrap();
        encoder.finish().unwrap();
    }

    /// Extract the gzip compressed data
    fn extract<I: Read, O: Write>(&self, input: I, mut output: O) {
        let mut decoder = GzDecoder::new(input);
        std::io::copy(&mut decoder, &mut output).unwrap();
    }
}
