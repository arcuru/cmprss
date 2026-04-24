use super::containers::total_input_bytes;
use crate::progress::{OutputTarget, ProgressArgs, ProgressReader, create_progress_bar};
use crate::utils::{
    CmprssInput, CmprssOutput, CommonArgs, CompressionLevelValidator, Compressor,
    DefaultCompressionValidator, ExtractedTarget, LevelArgs, Result,
};
use anyhow::{anyhow, bail};
use clap::Args;
use indicatif::ProgressBar;
use sevenz_rust2::{
    ArchiveEntry, ArchiveReader, ArchiveWriter, Password, decompress, encoder_options::Lzma2Options,
};
use std::fs::File;
use std::io::{self, Empty, Seek, SeekFrom, Write};
use std::path::Path;
use tempfile::tempfile;

#[derive(Args, Debug)]
pub struct SevenZArgs {
    #[clap(flatten)]
    pub common_args: CommonArgs,

    #[clap(flatten)]
    pub level_args: LevelArgs,

    #[clap(flatten)]
    pub progress_args: ProgressArgs,
}

#[derive(Clone)]
pub struct SevenZ {
    pub compression_level: i32,
    pub progress_args: ProgressArgs,
}

impl Default for SevenZ {
    fn default() -> Self {
        SevenZ {
            compression_level: DefaultCompressionValidator.default_level(),
            progress_args: ProgressArgs::default(),
        }
    }
}

impl SevenZ {
    pub fn new(args: &SevenZArgs) -> SevenZ {
        SevenZ {
            compression_level: args.level_args.resolve(&DefaultCompressionValidator),
            progress_args: args.progress_args,
        }
    }

    /// Extract a seekable 7z input with a byte-level progress bar keyed to
    /// the compressed archive size.
    fn decompress_seekable<R: io::Read + Seek>(
        &self,
        reader: R,
        size: u64,
        out_dir: &Path,
    ) -> Result {
        let bar = create_progress_bar(Some(size), self.progress_args.progress, OutputTarget::File);
        let reader = ProgressReader::new(reader, bar.clone());
        decompress(reader, out_dir)?;
        if let Some(b) = bar {
            b.finish();
        }
        Ok(())
    }

    /// Compress to the given seekable writer, walking path inputs ourselves
    /// so each file's read goes through `ProgressReader` sharing `bar`.
    fn compress_to_file<W: Write + Seek>(
        &self,
        input: CmprssInput,
        writer: W,
        bar: Option<&ProgressBar>,
    ) -> Result {
        let mut aw = ArchiveWriter::new(writer)?;
        let lzma = Lzma2Options::from_level(self.compression_level as u32);
        aw.set_content_methods(vec![lzma.into()]);

        match input {
            CmprssInput::Path(paths) => {
                for path in paths {
                    let name = path
                        .file_name()
                        .ok_or_else(|| anyhow!("input path has no file name: {:?}", path))?
                        .to_string_lossy()
                        .to_string();
                    if path.is_file() {
                        push_file_entry(&mut aw, &name, &path, bar)?;
                    } else if path.is_dir() {
                        push_dir_entries(&mut aw, &name, &path, bar)?;
                    } else {
                        bail!("7z does not support this file type");
                    }
                }
            }
            CmprssInput::Pipe(pipe) => {
                let entry = ArchiveEntry::new_file("archive");
                aw.push_archive_entry(entry, Some(pipe))?;
            }
            CmprssInput::Reader(_) => {
                bail!("7z does not accept an in-memory reader input");
            }
        }

        aw.finish()?;
        Ok(())
    }
}

impl Compressor for SevenZ {
    fn name(&self) -> &str {
        "7z"
    }

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
                let mut temp_file = tempfile()?;
                self.compress_to_file(input, &mut temp_file, None)?;
                temp_file.seek(SeekFrom::Start(0))?;
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
                if !out_dir.exists() {
                    std::fs::create_dir_all(out_dir)?;
                } else if !out_dir.is_dir() {
                    bail!("7z extraction output must be a directory");
                }

                match input {
                    CmprssInput::Path(paths) => {
                        if paths.len() != 1 {
                            bail!("7z extraction expects exactly one archive file");
                        }
                        let file = File::open(&paths[0])?;
                        let size = file.metadata()?.len();
                        self.decompress_seekable(file, size, out_dir)
                    }
                    CmprssInput::Pipe(mut pipe) => {
                        let mut temp_file = tempfile()?;
                        io::copy(&mut pipe, &mut temp_file)?;
                        temp_file.seek(SeekFrom::Start(0))?;
                        let size = temp_file.metadata()?.len();
                        self.decompress_seekable(temp_file, size, out_dir)
                    }
                    CmprssInput::Reader(_) => {
                        bail!(
                            "7z extraction does not accept an in-memory reader input (requires seekable input)"
                        )
                    }
                }
            }
            CmprssOutput::Pipe(_) => bail!("7z extraction to stdout is not supported"),
            CmprssOutput::Writer(mut writer) => match input {
                CmprssInput::Path(paths) => {
                    if paths.len() != 1 {
                        bail!("7z extraction expects exactly one archive file");
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
        let stdout = io::stdout();
        let mut out = stdout.lock();
        match input {
            CmprssInput::Path(paths) => {
                if paths.len() != 1 {
                    bail!("7z listing expects exactly one archive file");
                }
                let reader = ArchiveReader::open(&paths[0], Password::empty())?;
                for entry in &reader.archive().files {
                    writeln!(out, "{}", entry.name())?;
                }
            }
            CmprssInput::Pipe(mut pipe) => {
                let mut temp = tempfile()?;
                io::copy(&mut pipe, &mut temp)?;
                temp.seek(SeekFrom::Start(0))?;
                let reader = ArchiveReader::new(temp, Password::empty())?;
                for entry in &reader.archive().files {
                    writeln!(out, "{}", entry.name())?;
                }
            }
            CmprssInput::Reader(mut reader) => {
                let mut temp = tempfile()?;
                io::copy(&mut reader, &mut temp)?;
                temp.seek(SeekFrom::Start(0))?;
                let ar = ArchiveReader::new(temp, Password::empty())?;
                for entry in &ar.archive().files {
                    writeln!(out, "{}", entry.name())?;
                }
            }
        }
        Ok(())
    }
}

/// Push a single regular file as an archive entry, with reads flowing
/// through `ProgressReader` so they tick the shared bar.
fn push_file_entry<W: Write + Seek>(
    aw: &mut ArchiveWriter<W>,
    archive_name: &str,
    disk_path: &Path,
    bar: Option<&ProgressBar>,
) -> Result {
    let entry = ArchiveEntry::from_path(disk_path, archive_name.to_string());
    let file = File::open(disk_path)?;
    let reader = ProgressReader::new(file, bar.cloned());
    aw.push_archive_entry(entry, Some(reader))?;
    Ok(())
}

/// Push a directory entry, then recurse into its children. Mirrors the
/// layout that `push_source_path` would produce (entries named
/// `<dir>/<child>`), but gives us a read hook for each file.
fn push_dir_entries<W: Write + Seek>(
    aw: &mut ArchiveWriter<W>,
    archive_name: &str,
    disk_path: &Path,
    bar: Option<&ProgressBar>,
) -> Result {
    let entry = ArchiveEntry::from_path(disk_path, archive_name.to_string());
    aw.push_archive_entry::<Empty>(entry, None)?;
    for child in std::fs::read_dir(disk_path)? {
        let child = child?;
        let child_path = child.path();
        let child_name = format!("{}/{}", archive_name, child.file_name().to_string_lossy());
        if child_path.is_file() {
            push_file_entry(aw, &child_name, &child_path, bar)?;
        } else if child_path.is_dir() {
            push_dir_entries(aw, &child_name, &child_path, bar)?;
        }
        // Skip symlinks/other types — parity with prior behavior.
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

    #[test]
    fn test_sevenz_interface() {
        let compressor = SevenZ::default();
        test_compressor_interface(&compressor, "7z", Some("7z"));
    }

    #[test]
    fn test_sevenz_default_compression() -> Result {
        let compressor = SevenZ::default();
        test_compression(&compressor)
    }

    #[test]
    fn test_sevenz_fast_compression() -> Result {
        let fast_compressor = SevenZ {
            compression_level: 1,
            progress_args: ProgressArgs::default(),
        };
        test_compression(&fast_compressor)
    }

    #[test]
    fn test_sevenz_best_compression() -> Result {
        let best_compressor = SevenZ {
            compression_level: 9,
            progress_args: ProgressArgs::default(),
        };
        test_compression(&best_compressor)
    }

    #[test]
    fn test_directory_handling() -> Result {
        let compressor = SevenZ::default();
        let dir = assert_fs::TempDir::new()?;
        let file_path = dir.child("file.txt");
        file_path.write_str("directory test data")?;
        let working_dir = assert_fs::TempDir::new()?;
        let archive = working_dir.child("dir_archive.7z");
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
