use crate::progress::{copy_with_progress, ProgressArgs};
use crate::utils::{cmprss_error, CmprssInput, CmprssOutput, CommonArgs, Compressor};
use clap::Args;
use lz4_flex::frame::{FrameDecoder, FrameEncoder};
use std::fs::File;
use std::io::{self, BufReader, BufWriter, Read, Write};

#[derive(Args, Debug)]
pub struct Lz4Args {
    #[clap(flatten)]
    pub common_args: CommonArgs,

    #[clap(flatten)]
    pub progress_args: ProgressArgs,
}

#[derive(Default)]
pub struct Lz4 {
    pub progress_args: ProgressArgs,
}

impl Lz4 {
    pub fn new(args: &Lz4Args) -> Lz4 {
        Lz4 {
            progress_args: args.progress_args,
        }
    }
}

impl Compressor for Lz4 {
    /// The standard extension for the lz4 format.
    fn extension(&self) -> &str {
        "lz4"
    }

    /// Full name for lz4.
    fn name(&self) -> &str {
        "lz4"
    }

    /// Generate a default extracted filename
    /// lz4 does not support extracting to a directory, so we return a default filename
    fn default_extracted_filename(&self, in_path: &std::path::Path) -> String {
        // If the file has no extension, return a default filename
        if in_path.extension().is_none() {
            return "archive".to_string();
        }
        // Otherwise, return the filename without the extension
        in_path.file_stem().unwrap().to_str().unwrap().to_string()
    }

    /// Compress an input file or pipe to a lz4 archive
    fn compress(&self, input: CmprssInput, output: CmprssOutput) -> Result<(), io::Error> {
        if let CmprssOutput::Path(out_path) = &output {
            if out_path.is_dir() {
                return cmprss_error("LZ4 does not support compressing to a directory. Please specify an output file.");
            }
        }
        if let CmprssInput::Path(input_paths) = &input {
            for x in input_paths {
                if x.is_dir() {
                    return cmprss_error(
                        "LZ4 does not support compressing a directory. Please specify only files.",
                    );
                }
            }
        }
        let mut file_size = None;
        let mut input_stream: Box<dyn Read + Send> = match input {
            CmprssInput::Path(paths) => {
                if paths.len() > 1 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "Multiple input files not supported for lz4",
                    ));
                }
                let path = &paths[0];
                file_size = Some(std::fs::metadata(path)?.len());
                Box::new(BufReader::new(File::open(path)?))
            }
            CmprssInput::Pipe(stdin) => Box::new(BufReader::new(stdin)),
        };

        let output_stream: Box<dyn Write + Send> = match &output {
            CmprssOutput::Path(path) => Box::new(BufWriter::new(File::create(path)?)),
            CmprssOutput::Pipe(stdout) => Box::new(BufWriter::new(stdout)),
        };

        // Create a lz4 encoder
        let mut encoder = FrameEncoder::new(output_stream);

        // Copy the input to the encoder with progress reporting
        copy_with_progress(
            &mut input_stream,
            &mut encoder,
            self.progress_args.chunk_size.size_in_bytes,
            file_size,
            self.progress_args.progress,
            &output,
        )?;

        // Finish the encoder to ensure all data is written
        encoder.finish()?;

        Ok(())
    }

    /// Extract a lz4 archive to an output file or pipe
    fn extract(&self, input: CmprssInput, output: CmprssOutput) -> Result<(), io::Error> {
        if let CmprssOutput::Path(out_path) = &output {
            if out_path.is_dir() {
                return cmprss_error("LZ4 does not support extracting to a directory. Please specify an output file.");
            }
        }

        let input_stream: Box<dyn Read + Send> = match input {
            CmprssInput::Path(paths) => {
                if paths.len() > 1 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "Multiple input files not supported for lz4",
                    ));
                }
                let path = &paths[0];
                Box::new(BufReader::new(File::open(path)?))
            }
            CmprssInput::Pipe(stdin) => Box::new(BufReader::new(stdin)),
        };

        // Create a lz4 decoder
        let mut decoder = FrameDecoder::new(input_stream);

        let mut output_stream: Box<dyn Write + Send> = match &output {
            CmprssOutput::Path(path) => Box::new(BufWriter::new(File::create(path)?)),
            CmprssOutput::Pipe(stdout) => Box::new(BufWriter::new(stdout)),
        };

        // Copy the decoded data to the output with progress reporting
        copy_with_progress(
            &mut decoder,
            &mut output_stream,
            self.progress_args.chunk_size.size_in_bytes,
            None,
            self.progress_args.progress,
            &output,
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn roundtrip() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempdir()?;
        let input_path = dir.path().join("input.txt");
        let compressed_path = dir.path().join("input.txt.lz4");
        let output_path = dir.path().join("output.txt");

        // Create a test file
        let test_data = b"Hello, world! This is a test file for lz4 compression.";
        std::fs::write(&input_path, test_data)?;

        // Compress the file
        let lz4 = Lz4::default();
        lz4.compress(
            CmprssInput::Path(vec![input_path.clone()]),
            CmprssOutput::Path(compressed_path.clone()),
        )?;

        // Extract the file
        lz4.extract(
            CmprssInput::Path(vec![compressed_path]),
            CmprssOutput::Path(output_path.clone()),
        )?;

        // Verify the contents
        let output_data = std::fs::read(output_path)?;
        assert_eq!(test_data.to_vec(), output_data);

        Ok(())
    }
}
