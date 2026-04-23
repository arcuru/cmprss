use crate::utils::{CmprssInput, CmprssOutput, CommonArgs, Compressor, ExtractedTarget, Result};
use anyhow::bail;
use clap::Args;
use sevenz_rust2::{
    ArchiveEntry, ArchiveReader, ArchiveWriter, Password, decompress, decompress_file,
};
use std::fs::File;
use std::io::{self, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use tempfile::tempfile;

#[derive(Args, Debug)]
pub struct SevenZArgs {
    #[clap(flatten)]
    pub common_args: CommonArgs,
}

#[derive(Default, Clone)]
pub struct SevenZ {}

impl SevenZ {
    pub fn new(_args: &SevenZArgs) -> SevenZ {
        SevenZ {}
    }

    fn compress_to_file<W: Write + Seek>(&self, input: CmprssInput, writer: W) -> Result {
        let mut aw = ArchiveWriter::new(writer)?;

        match input {
            CmprssInput::Path(paths) => {
                for path in paths {
                    if path.is_file() {
                        aw.push_source_path(&path, |_| true)?;
                    } else if path.is_dir() {
                        add_directory_entries(&mut aw, &path)?;
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

    fn clone_boxed(&self) -> Box<dyn Compressor> {
        Box::new(self.clone())
    }

    fn default_extracted_target(&self) -> ExtractedTarget {
        ExtractedTarget::Directory
    }

    fn compress(&self, input: CmprssInput, output: CmprssOutput) -> Result {
        match output {
            CmprssOutput::Path(ref path) => {
                let file = File::create(path)?;
                self.compress_to_file(input, file)
            }
            CmprssOutput::Pipe(mut pipe) => {
                let mut temp_file = tempfile()?;
                self.compress_to_file(input, &mut temp_file)?;
                temp_file.seek(SeekFrom::Start(0))?;
                io::copy(&mut temp_file, &mut pipe)?;
                Ok(())
            }
            CmprssOutput::Writer(mut writer) => {
                let mut temp_file = tempfile()?;
                self.compress_to_file(input, &mut temp_file)?;
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
                        decompress_file(&paths[0], out_dir)?;
                        Ok(())
                    }
                    CmprssInput::Pipe(mut pipe) => {
                        let mut temp_file = tempfile()?;
                        io::copy(&mut pipe, &mut temp_file)?;
                        temp_file.seek(SeekFrom::Start(0))?;
                        decompress(temp_file, out_dir)?;
                        Ok(())
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

/// Archive the contents of `dir` under the directory's basename.
///
/// `push_source_path` strips the src prefix from each entry name, so to keep
/// the directory itself as a prefix in the archive (e.g. `indir/file.txt`
/// instead of `file.txt`), we pass the *parent* as the src and filter to just
/// `dir`'s subtree.
fn add_directory_entries<W: Write + Seek>(aw: &mut ArchiveWriter<W>, dir: &Path) -> Result {
    let abs_dir = std::path::absolute(dir).unwrap_or_else(|_| dir.to_path_buf());
    let base: PathBuf = abs_dir
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| abs_dir.clone());
    let base_filter = base.clone();
    let target = abs_dir.clone();
    aw.push_source_path(&base, move |p| {
        p == base_filter.as_path() || p.starts_with(&target)
    })?;
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
