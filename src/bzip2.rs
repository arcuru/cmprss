use crate::utils::*;
use bzip2::write::{BzDecoder, BzEncoder};
use bzip2::Compression;
use std::{
    fs::File,
    io::{self, Read, Write},
};

pub struct Bzip2 {
    pub level: u32, // 0-9
}

impl Default for Bzip2 {
    fn default() -> Self {
        Bzip2 { level: 6 }
    }
}

impl Compressor for Bzip2 {
    /// The standard extension for the bz2 format.
    fn extension(&self) -> &str {
        "bz2"
    }

    /// Full name for bz2.
    fn name(&self) -> &str {
        "bzip2"
    }

    /// Compress an input file or pipe to a bz2 archive
    fn compress(&self, input: CmprssInput, output: CmprssOutput) -> Result<(), io::Error> {
        let mut input_stream = match input {
            CmprssInput::Path(paths) => {
                if paths.len() > 1 {
                    return cmprss_error("only 1 file can be compressed at a time");
                }
                Box::new(File::open(paths[0].as_path())?)
            }
            CmprssInput::Pipe(pipe) => Box::new(pipe) as Box<dyn Read + Send>,
        };
        let output_stream: Box<dyn Write + Send> = match output {
            CmprssOutput::Path(path) => Box::new(File::create(path)?),
            CmprssOutput::Pipe(pipe) => Box::new(pipe) as Box<dyn Write + Send>,
        };
        let mut encoder = BzEncoder::new(output_stream, Compression::new(self.level));
        io::copy(&mut input_stream, &mut encoder)?;
        Ok(())
    }

    /// Extract a bz2 archive to a file or pipe
    fn extract(&self, input: CmprssInput, output: CmprssOutput) -> Result<(), io::Error> {
        let mut input_stream = match input {
            CmprssInput::Path(paths) => {
                if paths.len() > 1 {
                    return cmprss_error("only 1 file can be extracted at a time");
                }
                Box::new(File::open(paths[0].as_path())?)
            }
            CmprssInput::Pipe(pipe) => Box::new(pipe) as Box<dyn Read + Send>,
        };
        let output_stream: Box<dyn Write + Send> = match output {
            CmprssOutput::Path(path) => Box::new(File::create(path)?),
            CmprssOutput::Pipe(pipe) => Box::new(pipe) as Box<dyn Write + Send>,
        };
        let mut decoder = BzDecoder::new(output_stream);
        io::copy(&mut input_stream, &mut decoder)?;
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
        let compressor = Bzip2::default();

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
