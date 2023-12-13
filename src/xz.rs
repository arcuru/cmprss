use crate::{
    progress::{progress_bar, ProgressDisplay},
    utils::*,
};
use std::{
    fs::File,
    io::{self, Read, Write},
};
use xz2::write::{XzDecoder, XzEncoder};

pub struct Xz {
    pub level: u32,
    pub progress: ProgressDisplay,
    pub chunk_size: usize,
}

impl Default for Xz {
    fn default() -> Self {
        Xz {
            level: 6,
            progress: ProgressDisplay::Auto,
            chunk_size: 8192,
        }
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
        let output_stream: Box<dyn Write + Send> = match &output {
            CmprssOutput::Path(path) => Box::new(File::create(path)?),
            CmprssOutput::Pipe(pipe) => Box::new(pipe) as Box<dyn Write + Send>,
        };
        let mut encoder = XzEncoder::new(output_stream, self.level);
        let mut bar = progress_bar(file_size, self.progress, &output);
        if let Some(progress) = &mut bar {
            // Copy the input to the output in chunks so that we can update the progress bar
            let mut buffer = vec![0; self.chunk_size];
            loop {
                let bytes_read = input_stream.read(&mut buffer)?;
                if bytes_read == 0 {
                    break;
                }
                encoder.write_all(&buffer[..bytes_read])?;
                progress.update_input(encoder.total_in());
                progress.update_output(encoder.total_out());
            }
            encoder.flush()?;
            progress.update_output(encoder.total_out());
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
        let output_stream: Box<dyn Write + Send> = match &output {
            CmprssOutput::Path(path) => Box::new(File::create(path)?),
            CmprssOutput::Pipe(pipe) => Box::new(pipe) as Box<dyn Write + Send>,
        };
        let mut decoder = XzDecoder::new(output_stream);
        let mut bar = progress_bar(file_size, self.progress, &output);
        if let Some(progress) = &mut bar {
            // Copy the input to the output in chunks so that we can update the progress bar
            let mut buffer = vec![0; self.chunk_size];
            loop {
                let bytes_read = input_stream.read(&mut buffer)?;
                if bytes_read == 0 {
                    break;
                }
                decoder.write_all(&buffer[..bytes_read])?;
                progress.update_input(decoder.total_in());
                progress.update_output(decoder.total_out());
            }
            decoder.flush()?;
            progress.update_output(decoder.total_out());
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
