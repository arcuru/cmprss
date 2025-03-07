use crate::{
    progress::{copy_with_progress, ProgressArgs},
    utils::*,
};
use clap::Args;
use std::{
    fs::File,
    io::{self, Read, Write},
};
use xz2::read::XzDecoder;
use xz2::write::XzEncoder;

#[derive(Args, Debug)]
pub struct XzArgs {
    #[clap(flatten)]
    pub common_args: CommonArgs,

    #[clap(flatten)]
    progress_args: ProgressArgs,

    #[clap(flatten)]
    pub level_args: LevelArgs,
}

pub struct Xz {
    pub level: i32,
    pub progress_args: ProgressArgs,
}

impl Default for Xz {
    fn default() -> Self {
        let validator = DefaultCompressionValidator;
        Xz {
            level: validator.default_level(),
            progress_args: ProgressArgs::default(),
        }
    }
}

impl Xz {
    pub fn new(args: &XzArgs) -> Xz {
        let validator = DefaultCompressionValidator;
        let level = validator.validate_and_clamp_level(args.level_args.level.level);

        Xz {
            level,
            progress_args: args.progress_args,
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
        let mut encoder = XzEncoder::new(output_stream, self.level as u32);

        // Use the custom output function to handle progress bar updates
        copy_with_progress(
            &mut input_stream,
            &mut encoder,
            self.progress_args.chunk_size.size_in_bytes,
            file_size,
            self.progress_args.progress,
            &output,
        )?;

        Ok(())
    }

    fn extract(&self, input: CmprssInput, output: CmprssOutput) -> Result<(), io::Error> {
        let mut file_size = None;
        let input_stream: Box<dyn Read + Send> = match input {
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
        let mut output_stream: Box<dyn Write + Send> = match &output {
            CmprssOutput::Path(path) => Box::new(File::create(path)?),
            CmprssOutput::Pipe(pipe) => Box::new(pipe) as Box<dyn Write + Send>,
        };

        // Create an XZ decoder to decompress the input
        let mut decoder = XzDecoder::new(input_stream);

        // Use the custom output function to handle progress bar updates
        copy_with_progress(
            &mut decoder,
            &mut *output_stream,
            self.progress_args.chunk_size.size_in_bytes,
            file_size,
            self.progress_args.progress,
            &output,
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::*;

    /// Test the basic interface of the Xz compressor
    #[test]
    fn test_xz_interface() {
        let compressor = Xz::default();
        test_compressor_interface(&compressor, "xz", Some("xz"));
    }

    /// Test the default compression level
    #[test]
    fn test_xz_default_compression() -> Result<(), io::Error> {
        let compressor = Xz::default();
        test_compression(&compressor)
    }

    /// Test fast compression level
    #[test]
    fn test_xz_fast_compression() -> Result<(), io::Error> {
        let fast_compressor = Xz {
            level: 1,
            progress_args: ProgressArgs::default(),
        };
        test_compression(&fast_compressor)
    }

    /// Test best compression level
    #[test]
    fn test_xz_best_compression() -> Result<(), io::Error> {
        let best_compressor = Xz {
            level: 9,
            progress_args: ProgressArgs::default(),
        };
        test_compression(&best_compressor)
    }
}
