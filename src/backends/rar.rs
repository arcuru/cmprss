use std::io;
use std::path::Path;

use unrar::Archive;

use crate::utils::{finalize_unpack, Compressor, Format, CommonArgs, CmprssInput, CmprssOutput};

#[derive(Debug, Default, clap::Args, Clone)]
pub struct RarArgs {
    #[clap(flatten)]
    pub common_args: CommonArgs,
}

#[derive(Default)]
pub struct Rar {
    args: RarArgs,
}

impl Rar {
    pub fn new(_args: RarArgs) -> Self { // Mark args as unused for now
        Self::default() // Since args is not used, we can use default
    }
}

impl Compressor for Rar {
    fn name(&self) -> &str {
        "rar"
    }

    fn compress(&self, _input: CmprssInput, _output: CmprssOutput) -> Result<(), io::Error> {
        Err(io::Error::new(io::ErrorKind::Other, "RAR compression is not supported."))
    }

    fn extract(&self, input: CmprssInput, output: CmprssOutput) -> Result<(), io::Error> {
        let source_path = match input {
            CmprssInput::Path(paths) => {
                if paths.len() != 1 {
                    return Err(io::Error::new(io::ErrorKind::InvalidInput, "RAR extraction requires a single source file."));
                }
                paths.into_iter().next().unwrap()
            }
            CmprssInput::Pipe(_) => return Err(io::Error::new(io::ErrorKind::InvalidInput, "RAR extraction from pipe is not supported.")),
        };

        let destination_path = match output {
            CmprssOutput::Path(path) => path,
            CmprssOutput::Pipe(_) => return Err(io::Error::new(io::ErrorKind::InvalidInput, "RAR extraction to pipe is not supported.")),
        };

        let mut archive_cursor = Archive::new(&source_path)
            .open_for_processing()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to open RAR archive: {}", e)))?;

        loop {
            match archive_cursor.read_header() {
                Ok(Some(entry_reader)) => { // entry_reader is OpenArchive<Process, CursorBeforeFile>
                    let entry_info = entry_reader.entry();
                    let entry_path_string = entry_info.filename.to_string_lossy().into_owned();
                    let entry_path = Path::new(&entry_path_string);
                    let dest_path = destination_path.join(entry_path);

                    if entry_info.is_directory() {
                        std::fs::create_dir_all(&dest_path)
                            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to create directory '{}': {}", dest_path.display(), e)))?;
                        // After creating directory, we need to advance the cursor past this entry
                        archive_cursor = entry_reader.skip().map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to skip directory entry '{}': {}", entry_path.display(), e)))?;
                    } else {
                        if let Some(parent) = dest_path.parent() {
                            if !parent.exists() {
                                std::fs::create_dir_all(parent)
                                    .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to create parent directory '{}': {}", parent.display(), e)))?;
                            }
                        }
                        // entry_reader is the archive object itself, configured for the current entry.
                        // We use extract_to method on it.
                        archive_cursor = entry_reader.extract_to(&dest_path)
                            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to extract file '{}': {}", entry_path.display(), e)))?;
                    }
                    // After processing (extracting or skipping) the current file header,
                    // the next call to read_header() should advance to the next file.
                }
                Ok(None) => break, // End of archive
                Err(e) => return Err(io::Error::new(io::ErrorKind::Other, format!("Failed to read RAR entry header: {}", e))),
            }
        }
        finalize_unpack(&destination_path, Format::Rar);
        Ok(())
    }
}

// Separate implementation block for non-Compressor methods if any, or keep them here.
impl Rar {
    // This function is part of the Decompressor capabilities
    pub fn list_contents(&self, source: &Path) -> Result<Vec<String>, String> {
        let archive = Archive::new(source)
            .open_for_listing()
            .map_err(|e| format!("Failed to open RAR archive for listing: {}", e))?;


        let mut contents = Vec::new();
        for entry_result in archive {
            let entry = entry_result.map_err(|e| format!("Failed to read RAR entry: {}", e))?;
            contents.push(
                entry
                    .filename
                    .to_string_lossy()
                    .into_owned(),
            );
        }
        Ok(contents)
    }
}
