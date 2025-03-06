extern crate tar;

use clap::Args;
use std::fs::File;
use std::io::{self, Seek, SeekFrom, Write};
use std::path::Path;
use tar::{Archive, Builder};
use tempfile::tempfile;

use crate::utils::*;

#[derive(Args, Debug)]
pub struct TarArgs {
    #[clap(flatten)]
    pub common_args: CommonArgs,
}

#[derive(Default)]
pub struct Tar {}

impl Tar {
    pub fn new(_args: &TarArgs) -> Tar {
        Tar {}
    }
}

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
            CmprssOutput::Path(path) => {
                let file = File::create(path)?;
                self.compress_internal(input, Builder::new(file))
            }
            CmprssOutput::Pipe(mut pipe) => {
                // Create a temporary file to write the tar to
                let mut temp_file = tempfile()?;
                self.compress_internal(input, Builder::new(&mut temp_file))?;

                // Reset the file position to the beginning
                temp_file.seek(SeekFrom::Start(0))?;

                // Copy the temporary file to the pipe
                io::copy(&mut temp_file, &mut pipe)?;
                Ok(())
            }
        }
    }

    fn extract(&self, input: CmprssInput, output: CmprssOutput) -> Result<(), io::Error> {
        match output {
            CmprssOutput::Path(ref out_dir) => {
                // Create the output directory if it doesn't exist
                if !out_dir.exists() {
                    std::fs::create_dir_all(out_dir)?;
                } else if !out_dir.is_dir() {
                    return cmprss_error("tar extraction output must be a directory");
                }

                match input {
                    CmprssInput::Path(paths) => {
                        if paths.len() != 1 {
                            return cmprss_error("tar extraction expects a single archive file");
                        }
                        let file = File::open(&paths[0])?;
                        let mut archive = Archive::new(file);
                        archive.unpack(out_dir)
                    }
                    CmprssInput::Pipe(mut pipe) => {
                        // Create a temporary file to store the tar content
                        let mut temp_file = tempfile()?;

                        // Copy from pipe to temporary file
                        io::copy(&mut pipe, &mut temp_file)?;

                        // Reset the file position to the beginning
                        temp_file.seek(SeekFrom::Start(0))?;

                        // Extract from the temporary file
                        let mut archive = Archive::new(temp_file);
                        archive.unpack(out_dir)
                    }
                }
            }
            CmprssOutput::Pipe(_) => cmprss_error("tar extraction to stdout is not supported"),
        }
    }
}

impl Tar {
    /// Internal compress helper
    fn compress_internal<W: Write>(
        &self,
        input: CmprssInput,
        mut archive: Builder<W>,
    ) -> Result<(), io::Error> {
        match input {
            CmprssInput::Path(paths) => {
                for path in paths {
                    if path.is_file() {
                        archive.append_file(
                            path.file_name().unwrap(),
                            &mut File::open(path.as_path())?,
                        )?;
                    } else if path.is_dir() {
                        archive.append_dir_all(path.file_name().unwrap(), path.as_path())?;
                    } else {
                        return cmprss_error("unsupported file type for tar compression");
                    }
                }
            }
            CmprssInput::Pipe(mut pipe) => {
                // For pipe input, we'll create a single file named "archive"
                let mut temp_file = tempfile()?;
                io::copy(&mut pipe, &mut temp_file)?;
                temp_file.seek(SeekFrom::Start(0))?;
                archive.append_file("archive", &mut temp_file)?;
            }
        }
        archive.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_fs::prelude::*;
    use predicates::prelude::*;
    use std::path::PathBuf;

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

    #[test]
    fn roundtrip_directory() -> Result<(), Box<dyn std::error::Error>> {
        let compressor = Tar::default();
        let dir = assert_fs::TempDir::new()?;
        let file_path = dir.child("file.txt");
        file_path.write_str("garbage data for testing")?;
        let working_dir = assert_fs::TempDir::new()?;
        let archive = working_dir.child("dir_archive.tar");
        archive.assert(predicate::path::missing());

        compressor.compress(
            CmprssInput::Path(vec![dir.path().to_path_buf()]),
            CmprssOutput::Path(archive.path().to_path_buf()),
        )?;
        archive.assert(predicate::path::is_file());

        let extract_dir = working_dir.child("extracted");
        std::fs::create_dir_all(extract_dir.path())?;
        compressor.extract(
            CmprssInput::Path(vec![archive.path().to_path_buf()]),
            CmprssOutput::Path(extract_dir.path().to_path_buf()),
        )?;

        let dir_name: PathBuf = dir.path().file_name().unwrap().into();
        extract_dir
            .child(dir_name)
            .child("file.txt")
            .assert(predicate::path::eq_file(file_path.path()));
        Ok(())
    }
}
