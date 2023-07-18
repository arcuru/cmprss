use crate::utils::*;
use flate2::write::GzEncoder;
use flate2::{read::GzDecoder, Compression};
use std::fs::File;
use std::io::{self, Read, Write};

pub struct Gzip {
    pub compression_level: u32,
}

impl Default for Gzip {
    fn default() -> Self {
        Gzip {
            compression_level: 6,
        }
    }
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

    fn compress(&self, input: CmprssInput, output: CmprssOutput) -> Result<(), io::Error> {
        if let CmprssOutput::Path(out_path) = &output {
            if out_path.is_dir() {
                return cmprss_error("Gzip does not support compressing to a directory. Please specify an output file.");
            }
        }
        if let CmprssInput::Path(input_paths) = &input {
            for x in input_paths {
                if x.is_dir() {
                    return cmprss_error(
                        "Gzip does not support compressing a directory. Please specify only files.",
                    );
                }
            }
        }
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
                self.extract_internal(File::open(in_path[0].as_path())?, File::create(out_path)?)
            }
            (CmprssInput::Path(in_path), CmprssOutput::Pipe(out_pipe)) => {
                if in_path.len() > 1 {
                    return cmprss_error("only 1 archive can be extracted at a time");
                }
                self.extract_internal(File::open(in_path[0].as_path())?, out_pipe)
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

#[cfg(test)]
mod tests {
    use super::*;
    use assert_fs::prelude::*;
    use predicates::prelude::*;

    #[test]
    fn roundtrip() -> Result<(), Box<dyn std::error::Error>> {
        let compressor = Gzip::default();

        let file = assert_fs::NamedTempFile::new("test.txt")?;
        file.write_str("garbage data for testing")?;
        let working_dir = assert_fs::TempDir::new()?;
        let archive = working_dir.child("archive.".to_owned() + compressor.extension());
        archive.assert(predicate::path::missing());

        // Roundtrip compress/extract
        compressor.compress(
            CmprssInput::Path(vec![file.path().to_path_buf()]),
            CmprssOutput::Path(archive.path().to_path_buf()),
        )?;
        archive.assert(predicate::path::is_file());
        compressor.extract(
            CmprssInput::Path(vec![archive.path().to_path_buf()]),
            CmprssOutput::Path(working_dir.child("test.txt").path().to_path_buf()),
        )?;

        // Assert the files are identical
        working_dir
            .child("test.txt")
            .assert(predicate::path::eq_file(file.path()));

        Ok(())
    }
}
