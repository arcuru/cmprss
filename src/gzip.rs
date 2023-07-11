use crate::utils::*;
use flate2::write::GzEncoder;
use flate2::{read::GzDecoder, Compression};
use std::fs::File;
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

    fn compress(&self, input: CmprssInput, output: CmprssOutput) -> Result<(), io::Error> {
        match (input, output) {
            (CmprssInput::Path(in_path), CmprssOutput::Path(out_path)) => {
                let mut encoder = GzEncoder::new(
                    File::create(out_path)?,
                    Compression::new(self.compression_level),
                );
                for x in in_path {
                    std::io::copy(&mut File::open(x)?, &mut encoder)?;
                }
                encoder.finish()?;
                Ok(())
            }
            (CmprssInput::Path(in_path), CmprssOutput::Pipe(out_pipe)) => {
                let mut encoder =
                    GzEncoder::new(out_pipe, Compression::new(self.compression_level));
                for x in in_path {
                    std::io::copy(&mut File::open(x)?, &mut encoder)?;
                }
                encoder.finish()?;
                Ok(())
            }
            (CmprssInput::Pipe(in_pipe), CmprssOutput::Path(out_path)) => {
                self.compress_internal(in_pipe, File::create(out_path)?)
            }
            (CmprssInput::Pipe(in_pipe), CmprssOutput::Pipe(out_pipe)) => {
                self.compress_internal(in_pipe, out_pipe)
            }
        }
    }

    fn extract(&self, input: CmprssInput, output: CmprssOutput) -> Result<(), io::Error> {
        match (input, output) {
            (CmprssInput::Path(in_path), CmprssOutput::Path(out_path)) => {
                if in_path.len() > 1 {
                    return cmprss_error("only 1 archive can be extracted at a time");
                }
                self.extract_internal(File::open(in_path[0])?, File::create(out_path)?)
            }
            (CmprssInput::Path(in_path), CmprssOutput::Pipe(out_pipe)) => {
                if in_path.len() > 1 {
                    return cmprss_error("only 1 archive can be extracted at a time");
                }
                self.extract_internal(File::open(in_path[0])?, out_pipe)
            }
            (CmprssInput::Pipe(in_pipe), CmprssOutput::Path(out_path)) => {
                self.extract_internal(in_pipe, File::create(out_path)?)
            }
            (CmprssInput::Pipe(in_pipe), CmprssOutput::Pipe(out_pipe)) => {
                self.extract_internal(in_pipe, out_pipe)
            }
        }
    }
}

impl Gzip {
    /// Compress an input stream into a gzip archive.
    fn compress_internal<I: Read, O: Write>(
        &self,
        mut input: I,
        output: O,
    ) -> Result<(), io::Error> {
        let mut encoder = GzEncoder::new(output, Compression::new(self.compression_level));

        std::io::copy(&mut input, &mut encoder)?;
        encoder.finish()?;
        Ok(())
    }

    /// Extract the gzip compressed data
    fn extract_internal<I: Read, O: Write>(
        &self,
        input: I,
        mut output: O,
    ) -> Result<(), io::Error> {
        let mut decoder = GzDecoder::new(input);
        std::io::copy(&mut decoder, &mut output)?;
        Ok(())
    }
}
