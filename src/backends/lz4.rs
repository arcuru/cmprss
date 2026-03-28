use crate::progress::{ProgressArgs, copy_with_progress};
use crate::utils::{CmprssInput, CmprssOutput, CommonArgs, Compressor, cmprss_error};
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
            CmprssInput::Reader(reader) => reader.0,
        };

        if let CmprssOutput::Writer(writer) = output {
            let mut encoder = FrameEncoder::new(writer);
            io::copy(&mut input_stream, &mut encoder)?;
            encoder.finish()?;
        } else {
            let output_stream: Box<dyn Write + Send> = match &output {
                CmprssOutput::Path(path) => Box::new(BufWriter::new(File::create(path)?)),
                CmprssOutput::Pipe(stdout) => Box::new(BufWriter::new(stdout)),
                CmprssOutput::Writer(_) => unreachable!(),
            };
            let mut encoder = FrameEncoder::new(output_stream);
            copy_with_progress(
                &mut input_stream,
                &mut encoder,
                self.progress_args.chunk_size.size_in_bytes,
                file_size,
                self.progress_args.progress,
                &output,
            )?;
            encoder.finish()?;
        }

        Ok(())
    }

    /// Extract a lz4 archive to an output file or pipe
    fn extract(&self, input: CmprssInput, output: CmprssOutput) -> Result<(), io::Error> {
        if let CmprssOutput::Path(out_path) = &output {
            if out_path.is_dir() {
                return cmprss_error("LZ4 does not support extracting to a directory. Please specify an output file.");
            }
        }

        let mut file_size = None;
        let input_stream: Box<dyn Read + Send> = match input {
            CmprssInput::Path(paths) => {
                if paths.len() > 1 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "Multiple input files not supported for lz4 extraction",
                    ));
                }
                let path = &paths[0];
                file_size = Some(std::fs::metadata(path)?.len());
                Box::new(BufReader::new(File::open(path)?))
            }
            CmprssInput::Pipe(stdin) => Box::new(BufReader::new(stdin)),
            CmprssInput::Reader(reader) => reader.0,
        };

        // Create a lz4 decoder
        let mut decoder = FrameDecoder::new(input_stream);

        if let CmprssOutput::Writer(mut writer) = output {
            io::copy(&mut decoder, &mut writer)?;
        } else {
            let mut output_stream: Box<dyn Write + Send> = match &output {
                CmprssOutput::Path(path) => Box::new(BufWriter::new(File::create(path)?)),
                CmprssOutput::Pipe(stdout) => Box::new(BufWriter::new(stdout)),
                CmprssOutput::Writer(_) => unreachable!(),
            };
            copy_with_progress(
                &mut decoder,
                &mut output_stream,
                self.progress_args.chunk_size.size_in_bytes,
                file_size,
                self.progress_args.progress,
                &output,
            )?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::*;

    /// Test the basic interface of the Lz4 compressor
    #[test]
    fn test_lz4_interface() {
        let compressor = Lz4::default();
        test_compressor_interface(&compressor, "lz4", Some("lz4"));
    }

    /// Test the default compression level
    #[test]
    fn test_lz4_default_compression() -> Result<(), io::Error> {
        let compressor = Lz4::default();
        test_compression(&compressor)
    }
}
