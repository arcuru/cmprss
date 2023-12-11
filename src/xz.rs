use crate::{progress::Progress, utils::*};
use std::{
    fs::File,
    io::{self, Read, Write},
};
use xz2::write::{XzDecoder, XzEncoder};

pub struct Xz {
    pub level: u32,
}

impl Default for Xz {
    fn default() -> Self {
        Xz { level: 6 }
    }
}

impl Compressor for Xz {
    /// The standard extension for the xz format.
    fn extension(&self) -> &str {
        "xz"
    }

    /// Full name for xz.
    fn name(&self) -> &str {
        "xz"
    }

    fn compress(&self, input: CmprssInput, output: CmprssOutput) -> Result<(), io::Error> {
        let mut file_size = None;
        let mut input_stream = match input {
            CmprssInput::Path(paths) => {
                if paths.len() > 1 {
                    return cmprss_error("only 1 file can be compressed at a time");
                }
                let file = Box::new(File::open(paths[0].as_path())?);
                // Get the file size for the progress bar
                if let Ok(metadata) = file.metadata() {
                    file_size = Some(metadata.len());
                }
                file
            }
            CmprssInput::Pipe(pipe) => Box::new(pipe) as Box<dyn Read + Send>,
        };
        // We want to use the progress bar unless this is in the middle of a pipe
        let mut progress_bar = false;
        let output_stream: Box<dyn Write + Send> = match output {
            CmprssOutput::Path(path) => {
                progress_bar = true;
                Box::new(File::create(path)?)
            }
            CmprssOutput::Pipe(pipe) => Box::new(pipe) as Box<dyn Write + Send>,
        };
        let mut encoder = XzEncoder::new(output_stream, self.level);
        if progress_bar {
            let mut progress = Progress::new(file_size);
            // Copy the input to the output in 8k chunks
            let mut buffer = [0; 8192];
            loop {
                let bytes_read = input_stream.read(&mut buffer)?;
                if bytes_read == 0 {
                    break;
                }
                encoder.write_all(&buffer[..bytes_read])?;
                progress.update_input(encoder.total_in());
                progress.update_output(encoder.total_out());
            }
            progress.finish();
        } else {
            io::copy(&mut input_stream, &mut encoder)?;
        }
        Ok(())
    }

    fn extract(&self, input: CmprssInput, output: CmprssOutput) -> Result<(), io::Error> {
        let mut file_size = None;
        let mut input_stream = match input {
            CmprssInput::Path(paths) => {
                if paths.len() > 1 {
                    return cmprss_error("only 1 file can be extracted at a time");
                }
                let file = Box::new(File::open(paths[0].as_path())?);
                // Get the file size for the progress bar
                if let Ok(metadata) = file.metadata() {
                    file_size = Some(metadata.len());
                }
                file
            }
            CmprssInput::Pipe(pipe) => Box::new(pipe) as Box<dyn Read + Send>,
        };
        // We want to use the progress bar unless this is in the middle of a pipe
        let mut progress_bar = false;
        let output_stream: Box<dyn Write + Send> = match output {
            CmprssOutput::Path(path) => {
                progress_bar = true;
                Box::new(File::create(path)?)
            }
            CmprssOutput::Pipe(pipe) => Box::new(pipe) as Box<dyn Write + Send>,
        };
        let mut decoder = XzDecoder::new(output_stream);
        if progress_bar {
            let mut progress = Progress::new(file_size);
            // Copy the input to the output in 8k chunks
            let mut buffer = [0; 8192];
            loop {
                let bytes_read = input_stream.read(&mut buffer)?;
                if bytes_read == 0 {
                    break;
                }
                decoder.write_all(&buffer[..bytes_read])?;
                progress.update_input(decoder.total_in());
                progress.update_output(decoder.total_out());
            }
            progress.finish();
        } else {
            io::copy(&mut input_stream, &mut decoder)?;
        }
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
        let compressor = Xz::default();

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
