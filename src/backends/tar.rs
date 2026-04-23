extern crate tar;

use anyhow::bail;
use clap::Args;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom, Write};
use tar::{Archive, Builder};
use tempfile::tempfile;

use crate::utils::{CmprssInput, CmprssOutput, CommonArgs, Compressor, ExtractedTarget, Result};

#[derive(Args, Debug)]
pub struct TarArgs {
    #[clap(flatten)]
    pub common_args: CommonArgs,
}

#[derive(Default, Clone)]
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

    fn clone_boxed(&self) -> Box<dyn Compressor> {
        Box::new(self.clone())
    }

    /// Tar extracts to a directory by default
    fn default_extracted_target(&self) -> ExtractedTarget {
        ExtractedTarget::Directory
    }

    fn compress(&self, input: CmprssInput, output: CmprssOutput) -> Result {
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
            CmprssOutput::Writer(mut writer) => {
                let mut temp_file = tempfile()?;
                self.compress_internal(input, Builder::new(&mut temp_file))?;
                temp_file.seek(SeekFrom::Start(0))?;
                io::copy(&mut temp_file, &mut writer)?;
                Ok(())
            }
        }
    }

    fn extract(&self, input: CmprssInput, output: CmprssOutput) -> Result {
        match output {
            CmprssOutput::Path(ref out_dir) => {
                // Create the output directory if it doesn't exist
                if !out_dir.exists() {
                    std::fs::create_dir_all(out_dir)?;
                } else if !out_dir.is_dir() {
                    bail!("tar extraction output must be a directory");
                }

                match input {
                    CmprssInput::Path(paths) => {
                        if paths.len() != 1 {
                            bail!("tar extraction expects exactly one archive file");
                        }
                        let file = File::open(&paths[0])?;
                        let mut archive = Archive::new(file);
                        Ok(archive.unpack(out_dir)?)
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
                        Ok(archive.unpack(out_dir)?)
                    }
                    CmprssInput::Reader(reader) => {
                        let mut archive = Archive::new(reader.0);
                        archive.unpack(out_dir)?;
                        Ok(())
                    }
                }
            }
            CmprssOutput::Pipe(_) => bail!("tar extraction to stdout is not supported"),
            CmprssOutput::Writer(mut writer) => match input {
                CmprssInput::Path(paths) => {
                    if paths.len() != 1 {
                        bail!("tar extraction expects exactly one archive file");
                    }
                    let mut file = File::open(&paths[0])?;
                    io::copy(&mut file, &mut writer)?;
                    Ok(())
                }
                CmprssInput::Pipe(mut pipe) => {
                    io::copy(&mut pipe, &mut writer)?;
                    Ok(())
                }
                CmprssInput::Reader(mut reader) => {
                    io::copy(&mut reader, &mut writer)?;
                    Ok(())
                }
            },
        }
    }

    fn list(&self, input: CmprssInput) -> Result {
        let reader: Box<dyn Read> = match input {
            CmprssInput::Path(paths) => {
                if paths.len() != 1 {
                    bail!("tar listing expects exactly one archive file");
                }
                Box::new(File::open(&paths[0])?)
            }
            CmprssInput::Pipe(stdin) => Box::new(stdin),
            CmprssInput::Reader(reader) => reader.0,
        };
        let mut archive = Archive::new(reader);
        let stdout = io::stdout();
        let mut out = stdout.lock();
        for entry in archive.entries()? {
            let entry = entry?;
            let path = entry.path()?;
            writeln!(out, "{}", path.display())?;
        }
        Ok(())
    }
}

impl Tar {
    /// Internal compress helper
    fn compress_internal<W: Write>(&self, input: CmprssInput, mut archive: Builder<W>) -> Result {
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
                        bail!("tar does not support this file type");
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
            CmprssInput::Reader(_) => {
                bail!("tar does not accept an in-memory reader input");
            }
        }
        Ok(archive.finish()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::*;
    use assert_fs::prelude::*;
    use predicates::prelude::*;
    use std::path::PathBuf;

    /// Test the basic interface of the Tar compressor
    #[test]
    fn test_tar_interface() {
        let compressor = Tar::default();
        test_compressor_interface(&compressor, "tar", Some("tar"));
    }

    /// Test the default compression level
    #[test]
    fn test_tar_default_compression() -> Result {
        let compressor = Tar::default();
        test_compression(&compressor)
    }

    /// Test tar-specific functionality: directory handling
    #[test]
    fn test_directory_handling() -> Result {
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
