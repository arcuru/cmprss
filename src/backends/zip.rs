use super::containers::total_input_bytes;
use crate::progress::{OutputTarget, ProgressArgs, ProgressReader, create_progress_bar};
use crate::utils::{
    CmprssInput, CmprssOutput, CommonArgs, CompressionLevelValidator, Compressor,
    DefaultCompressionValidator, ExtractedTarget, LevelArgs, Result,
};
use anyhow::bail;
use clap::Args;
use indicatif::ProgressBar;
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

    #[clap(flatten)]
    pub level_args: LevelArgs,

    #[clap(flatten)]
    pub progress_args: ProgressArgs,
}

#[derive(Clone)]
pub struct Zip {
    pub compression_level: i32,
    pub progress_args: ProgressArgs,
}

impl Default for Zip {
    fn default() -> Self {
        Zip {
            compression_level: DefaultCompressionValidator.default_level(),
            progress_args: ProgressArgs::default(),
        }
    }
}

impl Zip {
    pub fn new(args: &ZipArgs) -> Zip {
        Zip {
            compression_level: args.level_args.resolve(&DefaultCompressionValidator),
            progress_args: args.progress_args,
        }
    }

    fn file_options(&self) -> FileOptions<'static, ()> {
        FileOptions::<()>::default()
            .compression_method(CompressionMethod::Deflated)
            .compression_level(Some(self.compression_level as i64))
            .large_file(true)
    }

    fn extract_seekable<R: std::io::Read + Seek>(
        &self,
        reader: R,
        size: u64,
        out_dir: &Path,
    ) -> Result {
        let bar = create_progress_bar(Some(size), self.progress_args.progress, OutputTarget::File);
        let reader = ProgressReader::new(reader, bar.clone());
        let mut archive = ZipArchive::new(reader)?;
        archive.extract(out_dir)?;
        if let Some(b) = bar {
            b.finish();
        }
        Ok(())
    }

    fn compress_to_file<W: Write + Seek>(
        &self,
        input: CmprssInput,
        writer: W,
        bar: Option<&ProgressBar>,
    ) -> Result {
        let mut zip_writer = ZipWriter::new(writer);
        let options = self.file_options();

        match input {
            CmprssInput::Path(paths) => {
                for path in paths {
                    if path.is_file() {
                        let name = path.file_name().unwrap().to_string_lossy();
                        zip_writer.start_file(name, options)?;
                        let f = File::open(&path)?;
                        let mut reader = ProgressReader::new(f, bar.cloned());
                        io::copy(&mut reader, &mut zip_writer)?;
                    } else if path.is_dir() {
                        // Use the directory as the base and add its contents
                        let base = path.parent().unwrap_or(&path);
                        add_directory(&mut zip_writer, base, &path, options, bar)?;
                    } else {
                        bail!("zip does not support this file type");
                    }
                }
            }
            CmprssInput::Pipe(mut pipe) => {
                // For pipe input, we'll create a single file named "archive"
                zip_writer.start_file("archive", options)?;
                io::copy(&mut pipe, &mut zip_writer)?;
            }
            CmprssInput::Reader(_) => {
                bail!("zip does not accept an in-memory reader input");
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

    fn clone_boxed(&self) -> Box<dyn Compressor> {
        Box::new(self.clone())
    }

    /// Zip extracts to a directory by default
    fn default_extracted_target(&self) -> ExtractedTarget {
        ExtractedTarget::Directory
    }

    fn compress(&self, input: CmprssInput, output: CmprssOutput) -> Result {
        match output {
            CmprssOutput::Path(ref path) => {
                let total = match &input {
                    CmprssInput::Path(paths) => Some(total_input_bytes(paths)),
                    _ => None,
                };
                let bar =
                    create_progress_bar(total, self.progress_args.progress, OutputTarget::File);
                let file = File::create(path)?;
                self.compress_to_file(input, file, bar.as_ref())?;
                if let Some(b) = bar {
                    b.finish();
                }
                Ok(())
            }
            CmprssOutput::Pipe(mut pipe) => {
                // Create a temporary file to write the zip to
                let mut temp_file = tempfile()?;
                self.compress_to_file(input, &mut temp_file, None)?;

                // Reset the file position to the beginning
                temp_file.seek(SeekFrom::Start(0))?;

                // Copy the temporary file to the pipe
                io::copy(&mut temp_file, &mut pipe)?;
                Ok(())
            }
            CmprssOutput::Writer(mut writer) => {
                let mut temp_file = tempfile()?;
                self.compress_to_file(input, &mut temp_file, None)?;
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
                    bail!("zip extraction output must be a directory");
                }

                match input {
                    CmprssInput::Path(paths) => {
                        if paths.len() != 1 {
                            bail!("zip extraction expects exactly one archive file");
                        }
                        let file = File::open(&paths[0])?;
                        let size = file.metadata()?.len();
                        self.extract_seekable(file, size, out_dir)
                    }
                    CmprssInput::Pipe(mut pipe) => {
                        // Create a temporary file to store the zip content
                        let mut temp_file = tempfile()?;

                        // Copy from pipe to temporary file
                        io::copy(&mut pipe, &mut temp_file)?;

                        // Reset the file position to the beginning
                        temp_file.seek(SeekFrom::Start(0))?;
                        let size = temp_file.metadata()?.len();
                        self.extract_seekable(temp_file, size, out_dir)
                    }
                    CmprssInput::Reader(_) => {
                        bail!(
                            "zip extraction does not accept an in-memory reader input (requires seekable input)"
                        )
                    }
                }
            }
            CmprssOutput::Pipe(_) => bail!("zip extraction to stdout is not supported"),
            CmprssOutput::Writer(mut writer) => match input {
                CmprssInput::Path(paths) => {
                    if paths.len() != 1 {
                        bail!("zip extraction expects exactly one archive file");
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
        // ZipArchive requires a seekable reader. For non-path inputs we must
        // buffer into a tempfile first.
        let stdout = io::stdout();
        let mut out = stdout.lock();
        match input {
            CmprssInput::Path(paths) => {
                if paths.len() != 1 {
                    bail!("zip listing expects exactly one archive file");
                }
                let archive = ZipArchive::new(File::open(&paths[0])?)?;
                for name in archive.file_names() {
                    writeln!(out, "{}", name)?;
                }
            }
            CmprssInput::Pipe(mut pipe) => {
                let mut temp = tempfile()?;
                io::copy(&mut pipe, &mut temp)?;
                temp.seek(SeekFrom::Start(0))?;
                let archive = ZipArchive::new(temp)?;
                for name in archive.file_names() {
                    writeln!(out, "{}", name)?;
                }
            }
            CmprssInput::Reader(mut reader) => {
                let mut temp = tempfile()?;
                io::copy(&mut reader, &mut temp)?;
                temp.seek(SeekFrom::Start(0))?;
                let archive = ZipArchive::new(temp)?;
                for name in archive.file_names() {
                    writeln!(out, "{}", name)?;
                }
            }
        }
        Ok(())
    }
}

fn add_directory<W: Write + Seek>(
    zip: &mut ZipWriter<W>,
    base: &Path,
    path: &Path,
    options: FileOptions<'static, ()>,
    bar: Option<&ProgressBar>,
) -> Result {
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
            zip.start_file(name, options)?;
            let f = File::open(&entry_path)?;
            let mut reader = ProgressReader::new(f, bar.cloned());
            io::copy(&mut reader, zip)?;
        } else if entry_path.is_dir() {
            // Ensure directory entry ends with '/'
            let dir_name = name.clone() + "/";
            zip.add_directory(dir_name, options)?;
            add_directory(zip, base, &entry_path, options, bar)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::*;
    use assert_fs::prelude::*;
    use predicates::prelude::*;
    use std::path::PathBuf;

    /// Test the basic interface of the Zip compressor
    #[test]
    fn test_zip_interface() {
        let compressor = Zip::default();
        test_compressor_interface(&compressor, "zip", Some("zip"));
    }

    /// Test the default compression level
    #[test]
    fn test_zip_default_compression() -> Result {
        let compressor = Zip::default();
        test_compression(&compressor)
    }

    /// Test fast compression level
    #[test]
    fn test_zip_fast_compression() -> Result {
        let fast_compressor = Zip {
            compression_level: 1,
            progress_args: ProgressArgs::default(),
        };
        test_compression(&fast_compressor)
    }

    /// Test best compression level
    #[test]
    fn test_zip_best_compression() -> Result {
        let best_compressor = Zip {
            compression_level: 9,
            progress_args: ProgressArgs::default(),
        };
        test_compression(&best_compressor)
    }

    /// Test zip-specific functionality: directory handling
    #[test]
    fn test_directory_handling() -> Result {
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
