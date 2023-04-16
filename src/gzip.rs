use crate::utils::*;
use flate2::write::GzEncoder;
use flate2::{read::GzDecoder, Compression};
use std::io::{self, Read, Write};

pub struct Gzip {
    pub compression_level: u32,
    pub common_args: CmprssCommonArgs,
}

impl Compressor for Gzip {
    /// The standard extension for the gzip format.
    fn extension(&self) -> &str {
        "gz"
    }

    /// Full name for gzip.
    fn name(&self) -> &str {
        "gzip"
    }

    fn common_args(&self) -> &CmprssCommonArgs {
        &self.common_args
    }

    /// Compress an input stream into a gzip archive.
    fn compress<I: Read, O: Write>(&self, mut input: I, output: O) -> Result<(), io::Error> {
        let mut encoder = GzEncoder::new(output, Compression::new(self.compression_level));

        std::io::copy(&mut input, &mut encoder)?;
        encoder.finish()?;
        Ok(())
    }

    /// Extract the gzip compressed data
    fn extract<I: Read, O: Write>(&self, input: I, mut output: O) -> Result<(), io::Error> {
        let mut decoder = GzDecoder::new(input);
        std::io::copy(&mut decoder, &mut output)?;
        Ok(())
    }
}
