use crate::utils::*;
use clap::Args;
use std::fs::File;
use std::io::{self, Seek, SeekFrom, Write};
use std::path::Path;
use tempfile::tempfile;
use zip::read::ZipArchive;
use zip::write::FileOptions;
use zip::{CompressionMethod, ZipWriter};

#[derive(Args, Debug)]
pub struct ZipArgs {
    #[clap(flatten)]
    pub common_args: CommonArgs,
}

#[derive(Default)]
pub struct Zip {}

impl Zip {
    pub fn new(_args: &ZipArgs) -> Zip {
        Zip {}
    }

    fn compress_to_file<W: Write + Seek>(
        &self,
        input: CmprssInput,
        writer: W,
    ) -> Result<(), io::Error> {
        let mut zip_writer = ZipWriter::new(writer);
        let options = FileOptions::default().compression_method(CompressionMethod::Deflated);

        match input {
            CmprssInput::Path(paths) => {
                for path in paths {
                    if path.is_file() {
                        let name = path.file_name().unwrap().to_string_lossy();
                        zip_writer.start_file(name, options)?;
                        let mut f = File::open(&path)?;
                        io::copy(&mut f, &mut zip_writer)?;
                    } else if path.is_dir() {
                        // Use the directory as the base and add its contents
                        let base = path.parent().unwrap_or(&path);
                        add_directory(&mut zip_writer, base, &path)?;
                    } else {
                        return cmprss_error("unsupported file type for zip compression");
                    }
                }
            }
            CmprssInput::Pipe(mut pipe) => {
                // For pipe input, we'll create a single file named "archive"
                zip_writer.start_file("archive", options)?;
                io::copy(&mut pipe, &mut zip_writer)?;
            }
        }

        zip_writer.finish()?;
        Ok(())
    }
}

impl Compressor for Zip {
    fn name(&self) -> &str {
        "zip"
    }

    fn default_extracted_filename(&self, in_path: &Path) -> String {
        if let Some(stem) = in_path.file_stem() {
            stem.to_string_lossy().into_owned()
        } else {
            ".".to_string()
        }
    }

    fn compress(&self, input: CmprssInput, output: CmprssOutput) -> Result<(), io::Error> {
        match output {
            CmprssOutput::Path(ref path) => {
                let file = File::create(path)?;
                self.compress_to_file(input, file)
            }
            CmprssOutput::Pipe(mut pipe) => {
                // Create a temporary file to write the zip to
                let mut temp_file = tempfile()?;
                self.compress_to_file(input, &mut temp_file)?;

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
                    return cmprss_error("zip extraction output must be a directory");
                }

                match input {
                    CmprssInput::Path(paths) => {
                        if paths.len() != 1 {
                            return cmprss_error("zip extraction expects a single archive file");
                        }
                        let file = File::open(&paths[0])?;
                        let mut archive = ZipArchive::new(file)?;
                        archive
                            .extract(out_dir)
                            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
                    }
                    CmprssInput::Pipe(mut pipe) => {
                        // Create a temporary file to store the zip content
                        let mut temp_file = tempfile()?;

                        // Copy from pipe to temporary file
                        io::copy(&mut pipe, &mut temp_file)?;

                        // Reset the file position to the beginning
                        temp_file.seek(SeekFrom::Start(0))?;

                        // Extract from the temporary file
                        let mut archive = ZipArchive::new(temp_file)?;
                        archive
                            .extract(out_dir)
                            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
                    }
                }
            }
            CmprssOutput::Pipe(_) => cmprss_error("zip extraction to stdout is not supported"),
        }
    }
}

fn add_directory<W: Write + Seek>(
    zip: &mut ZipWriter<W>,
    base: &Path,
    path: &Path,
) -> Result<(), io::Error> {
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let entry_path = entry.path();
        // Get relative path for archive entry
        let name = entry_path
            .strip_prefix(base)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        if entry_path.is_file() {
            let options = FileOptions::default().compression_method(CompressionMethod::Deflated);
            zip.start_file(name, options)?;
            let mut f = File::open(&entry_path)?;
            io::copy(&mut f, zip)?;
        } else if entry_path.is_dir() {
            // Ensure directory entry ends with '/'
            let dir_name = name.clone() + "/";
            zip.add_directory(
                dir_name,
                FileOptions::default().compression_method(CompressionMethod::Deflated),
            )?;
            add_directory(zip, base, &entry_path)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_fs::prelude::*;
    use predicates::prelude::*;
    use std::path::PathBuf;

    #[test]
    fn roundtrip_file() -> Result<(), Box<dyn std::error::Error>> {
        let compressor = Zip::default();
        let file = assert_fs::NamedTempFile::new("test.txt")?;
        file.write_str("test data for zip")?;
        let working_dir = assert_fs::TempDir::new()?;
        let archive = working_dir.child("archive.zip");
        archive.assert(predicate::path::missing());

        compressor.compress(
            CmprssInput::Path(vec![file.path().to_path_buf()]),
            CmprssOutput::Path(archive.path().to_path_buf()),
        )?;
        archive.assert(predicate::path::is_file());

        let extract_dir = working_dir.child("out");
        std::fs::create_dir_all(extract_dir.path())?;
        compressor.extract(
            CmprssInput::Path(vec![archive.path().to_path_buf()]),
            CmprssOutput::Path(extract_dir.path().to_path_buf()),
        )?;
        extract_dir
            .child("test.txt")
            .assert(predicate::path::eq_file(file.path()));
        Ok(())
    }

    #[test]
    fn roundtrip_directory() -> Result<(), Box<dyn std::error::Error>> {
        let compressor = Zip::default();
        let dir = assert_fs::TempDir::new()?;
        let file_path = dir.child("file.txt");
        file_path.write_str("directory test data")?;
        let working_dir = assert_fs::TempDir::new()?;
        let archive = working_dir.child("dir_archive.zip");
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
        // When extracting a directory from a zip, the directory name is included in the path
        // Since the archive stores the entire directory, the extracted file is contained in the directory
        let dir_name: PathBuf = dir.path().file_name().unwrap().into();
        extract_dir
            .child(dir_name)
            .child("file.txt")
            .assert(predicate::path::eq_file(file_path.path()));
        Ok(())
    }
}
