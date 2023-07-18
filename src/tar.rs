extern crate tar;

use std::fs::File;
use std::io::{self, Read, Write};
use std::path::Path;
use tar::{Archive, Builder};

use crate::utils::*;

#[derive(Default)]
pub struct Tar {}

impl Compressor for Tar {
    /// Full name for tar, also used for extension
    fn name(&self) -> &str {
        "tar"
    }

    /// Tar extraction needs to specify the directory, so use the current directory
    fn default_extracted_filename(&self, _in_path: &Path) -> String {
        ".".to_string()
    }

    fn compress(&self, input: CmprssInput, output: CmprssOutput) -> Result<(), io::Error> {
        match output {
            CmprssOutput::Pipe(pipe) => self.compress_internal(input, Builder::new(pipe)),
            CmprssOutput::Path(path) => {
                self.compress_internal(input, Builder::new(File::create(path)?))
            }
        }
    }

    fn extract(&self, input: CmprssInput, output: CmprssOutput) -> Result<(), io::Error> {
        match input {
            CmprssInput::Path(paths) => {
                if paths.len() > 1 {
                    return cmprss_error("only 1 archive can be extracted at a time");
                }
                self.extract_internal(Archive::new(File::open(paths[0].as_path())?), output)
            }
            CmprssInput::Pipe(pipe) => self.extract_internal(Archive::new(pipe), output),
        }
    }
}

impl Tar {
    /// Internal extract helper
    fn extract_internal<R: Read>(
        &self,
        mut archive: Archive<R>,
        output: CmprssOutput,
    ) -> Result<(), io::Error> {
        let out_path = match output {
            CmprssOutput::Pipe(_) => {
                return cmprss_error("error: tar does not support stdout as extract output")
            }
            CmprssOutput::Path(path) => path,
        };
        if !out_path.is_dir() {
            return cmprss_error("error: tar can only extract to a directory");
        }
        archive.unpack(out_path)
    }

    /// Internal compress helper
    fn compress_internal<W: Write>(
        &self,
        input: CmprssInput,
        mut archive: Builder<W>,
    ) -> Result<(), io::Error> {
        let input_files = match input {
            CmprssInput::Path(paths) => paths,
            CmprssInput::Pipe(_) => {
                return cmprss_error("error: tar does not support stdin as input")
            }
        };
        for in_file in input_files {
            if in_file.is_file() {
                archive.append_file(
                    in_file.file_name().unwrap(),
                    &mut File::open(in_file.as_path())?,
                )?;
            } else if in_file.is_dir() {
                archive.append_dir_all(in_file.file_name().unwrap(), in_file.as_path())?;
            } else {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "unknown file type",
                ));
            }
        }
        archive.finish()
    }
}

// TODO: Tests will be largely the same for all Compressors, should be able to combine
#[cfg(test)]
mod tests {
    use super::*;
    use assert_fs::prelude::*;
    use predicates::prelude::*;

    #[test]
    fn roundtrip() -> Result<(), Box<dyn std::error::Error>> {
        let compressor = Tar::default();

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
            CmprssOutput::Path(working_dir.path().to_path_buf()),
        )?;

        // Assert the files are identical
        working_dir
            .child("test.txt")
            .assert(predicate::path::eq_file(file.path()));

        Ok(())
    }
}
