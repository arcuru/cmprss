extern crate tar;

use anyhow::{anyhow, bail};
use clap::Args;
use indicatif::ProgressBar;
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::Path;
use tar::{Archive, Builder, EntryType, Header};
use tempfile::tempfile;

use super::containers::total_input_bytes;
use crate::progress::{OutputTarget, ProgressArgs, ProgressReader, create_progress_bar};
use crate::utils::{CmprssInput, CmprssOutput, CommonArgs, Compressor, ExtractedTarget, Result};

#[derive(Args, Debug)]
pub struct TarArgs {
    #[clap(flatten)]
    pub common_args: CommonArgs,

    #[clap(flatten)]
    pub progress_args: ProgressArgs,
}

#[derive(Default, Clone)]
pub struct Tar {
    pub progress_args: ProgressArgs,
}

impl Tar {
    pub fn new(args: &TarArgs) -> Tar {
        Tar {
            progress_args: args.progress_args,
        }
    }
}

impl Compressor for Tar {
    /// Full name for tar, also used for extension
    fn name(&self) -> &str {
        "tar"
    }

    /// Tar extracts to a directory by default
    fn default_extracted_target(&self) -> ExtractedTarget {
        ExtractedTarget::Directory
    }

    fn compress(&self, input: CmprssInput, output: CmprssOutput) -> Result {
        match output {
            CmprssOutput::Path(path) => {
                let total = match &input {
                    CmprssInput::Path(paths) => Some(total_input_bytes(paths)),
                    _ => None,
                };
                let bar =
                    create_progress_bar(total, self.progress_args.progress, OutputTarget::File);
                let file = File::create(path)?;
                self.compress_internal(input, Builder::new(file), bar.as_ref())?;
                if let Some(b) = bar {
                    b.finish();
                }
                Ok(())
            }
            CmprssOutput::Pipe(mut pipe) => {
                // Create a temporary file to write the tar to
                let mut temp_file = tempfile()?;
                self.compress_internal(input, Builder::new(&mut temp_file), None)?;

                // Reset the file position to the beginning
                temp_file.seek(SeekFrom::Start(0))?;

                // Copy the temporary file to the pipe
                io::copy(&mut temp_file, &mut pipe)?;
                Ok(())
            }
            CmprssOutput::Writer(mut writer) => {
                // Pipeline-internal: tar is the innermost stage, writing into an
                // in-memory pipe feeding the outer codec(s). We still own the
                // progress bar because only tar sees the real input bytes; outer
                // stages suppress their bar (their input size is unknown).
                let total = match &input {
                    CmprssInput::Path(paths) => Some(total_input_bytes(paths)),
                    _ => None,
                };
                let bar =
                    create_progress_bar(total, self.progress_args.progress, OutputTarget::File);
                let mut temp_file = tempfile()?;
                self.compress_internal(input, Builder::new(&mut temp_file), bar.as_ref())?;
                temp_file.seek(SeekFrom::Start(0))?;
                io::copy(&mut temp_file, &mut writer)?;
                if let Some(b) = bar {
                    b.finish();
                }
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
                        let size = file.metadata()?.len();
                        self.unpack_with_progress(file, Some(size), out_dir)
                    }
                    CmprssInput::Pipe(mut pipe) => {
                        // Create a temporary file to store the tar content
                        let mut temp_file = tempfile()?;

                        // Copy from pipe to temporary file
                        io::copy(&mut pipe, &mut temp_file)?;

                        // Reset the file position to the beginning
                        temp_file.seek(SeekFrom::Start(0))?;
                        let size = temp_file.metadata()?.len();
                        self.unpack_with_progress(temp_file, Some(size), out_dir)
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

    fn append(&self, input: CmprssInput, output: CmprssOutput) -> Result {
        let path = match output {
            CmprssOutput::Path(p) => p,
            _ => bail!("tar append requires the archive path as the output target"),
        };
        if !path.is_file() {
            bail!("tar append target must be an existing file: {:?}", path);
        }

        // Locate the offset just past the last entry's data (512-byte padded)
        // so we can truncate off the trailing zero blocks and resume writing
        // entries from there. Using the iterator is cheap: tar entries carry
        // their own position, so we walk headers without reading file data.
        let end_of_entries = {
            let reader = File::open(&path)?;
            let mut archive = Archive::new(reader);
            let mut end: u64 = 0;
            for entry in archive.entries()? {
                let entry = entry?;
                let file_pos = entry.raw_file_position();
                let size = entry.size();
                // Round up to the next 512-byte block boundary.
                let padded = size.div_ceil(512) * 512;
                end = file_pos + padded;
            }
            end
        };

        let mut file = OpenOptions::new().read(true).write(true).open(&path)?;
        // Truncate any trailing end-of-archive zero blocks so the new entries
        // start at `end_of_entries` and Builder::finish writes fresh ones.
        file.set_len(end_of_entries)?;
        file.seek(SeekFrom::Start(end_of_entries))?;

        let total = match &input {
            CmprssInput::Path(paths) => Some(total_input_bytes(paths)),
            _ => None,
        };
        let bar = create_progress_bar(total, self.progress_args.progress, OutputTarget::File);
        self.compress_internal(input, Builder::new(file), bar.as_ref())?;
        if let Some(b) = bar {
            b.finish();
        }
        Ok(())
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
    /// Internal compress helper. When `bar` is `Some`, recursively walks
    /// path inputs ourselves (rather than using `Builder::append_dir_all`)
    /// so every file read runs through `ProgressReader`, sharing a single
    /// bar across all entries.
    fn compress_internal<W: Write>(
        &self,
        input: CmprssInput,
        mut archive: Builder<W>,
        bar: Option<&ProgressBar>,
    ) -> Result {
        match input {
            CmprssInput::Path(paths) => {
                for path in paths {
                    let name = path
                        .file_name()
                        .ok_or_else(|| anyhow!("input path has no file name: {:?}", path))?;
                    if path.is_file() {
                        append_file_entry(&mut archive, Path::new(name), &path, bar)?;
                    } else if path.is_dir() {
                        append_dir_entry(&mut archive, Path::new(name), &path, bar)?;
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

    fn unpack_with_progress<R: Read>(
        &self,
        reader: R,
        size: Option<u64>,
        out_dir: &Path,
    ) -> Result {
        let bar = create_progress_bar(size, self.progress_args.progress, OutputTarget::File);
        let reader = ProgressReader::new(reader, bar.clone());
        let mut archive = Archive::new(reader);
        archive.unpack(out_dir)?;
        if let Some(b) = bar {
            b.finish();
        }
        Ok(())
    }
}

/// Append one regular file to the tar archive, wrapping reads in a
/// `ProgressReader` that ticks the shared bar.
fn append_file_entry<W: Write>(
    archive: &mut Builder<W>,
    archive_name: &Path,
    disk_path: &Path,
    bar: Option<&ProgressBar>,
) -> Result {
    let mut file = File::open(disk_path)?;
    let meta = file.metadata()?;
    let mut header = Header::new_gnu();
    header.set_metadata(&meta);
    header.set_size(meta.len());
    let reader = ProgressReader::new(&mut file, bar.cloned());
    archive.append_data(&mut header, archive_name, reader)?;
    Ok(())
}

/// Write the directory header, then recurse into its children.
fn append_dir_entry<W: Write>(
    archive: &mut Builder<W>,
    archive_name: &Path,
    disk_path: &Path,
    bar: Option<&ProgressBar>,
) -> Result {
    let meta = std::fs::metadata(disk_path)?;
    let mut header = Header::new_gnu();
    header.set_metadata(&meta);
    header.set_entry_type(EntryType::Directory);
    header.set_size(0);
    archive.append_data(&mut header, archive_name, io::empty())?;
    for entry in std::fs::read_dir(disk_path)? {
        let entry = entry?;
        let child_archive = archive_name.join(entry.file_name());
        let child_disk = entry.path();
        if child_disk.is_file() {
            append_file_entry(archive, &child_archive, &child_disk, bar)?;
        } else if child_disk.is_dir() {
            append_dir_entry(archive, &child_archive, &child_disk, bar)?;
        }
        // Skip symlinks/other types; they weren't handled before either.
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

    /// Append new entries into an existing tar and confirm both old and new
    /// entries extract correctly.
    #[test]
    fn test_append_adds_entries() -> Result {
        let compressor = Tar::default();
        let working_dir = assert_fs::TempDir::new()?;

        let original = working_dir.child("original.txt");
        original.write_str("original contents")?;
        let extra = working_dir.child("extra.txt");
        extra.write_str("appended contents")?;

        let archive = working_dir.child("archive.tar");
        compressor.compress(
            CmprssInput::Path(vec![original.path().to_path_buf()]),
            CmprssOutput::Path(archive.path().to_path_buf()),
        )?;
        let size_before = std::fs::metadata(archive.path())?.len();

        compressor.append(
            CmprssInput::Path(vec![extra.path().to_path_buf()]),
            CmprssOutput::Path(archive.path().to_path_buf()),
        )?;
        let size_after = std::fs::metadata(archive.path())?.len();
        assert!(
            size_after > size_before,
            "archive did not grow after append: {size_before} -> {size_after}",
        );

        let extract_dir = working_dir.child("extracted");
        std::fs::create_dir_all(extract_dir.path())?;
        compressor.extract(
            CmprssInput::Path(vec![archive.path().to_path_buf()]),
            CmprssOutput::Path(extract_dir.path().to_path_buf()),
        )?;

        extract_dir
            .child("original.txt")
            .assert(predicate::path::eq_file(original.path()));
        extract_dir
            .child("extra.txt")
            .assert(predicate::path::eq_file(extra.path()));
        Ok(())
    }

    /// Appending to a missing target must error rather than silently creating
    /// a new archive.
    #[test]
    fn test_append_missing_target_errors() {
        let compressor = Tar::default();
        let working_dir = assert_fs::TempDir::new().unwrap();
        let extra = working_dir.child("extra.txt");
        extra.write_str("x").unwrap();
        let missing = working_dir.child("nope.tar");

        let err = compressor
            .append(
                CmprssInput::Path(vec![extra.path().to_path_buf()]),
                CmprssOutput::Path(missing.path().to_path_buf()),
            )
            .expect_err("append to a missing archive should error");
        assert!(err.to_string().contains("must be an existing file"));
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
