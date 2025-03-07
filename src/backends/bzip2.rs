use crate::{
    progress::{copy_with_progress, ProgressArgs},
    utils::{
        cmprss_error, CmprssInput, CmprssOutput, CommonArgs, CompressionLevelValidator, Compressor,
        ExtractedTarget, LevelArgs,
    },
};
use bzip2::write::{BzDecoder, BzEncoder};
use bzip2::Compression;
use clap::Args;
use std::{
    fs::File,
    io::{self, Read, Write},
};

/// BZip2-specific compression validator (1-9 range)
#[derive(Debug, Clone, Copy)]
pub struct Bzip2CompressionValidator;

impl CompressionLevelValidator for Bzip2CompressionValidator {
    fn min_level(&self) -> i32 {
        1
    }
    fn max_level(&self) -> i32 {
        9
    }
    fn default_level(&self) -> i32 {
        9
    }

    fn name_to_level(&self, name: &str) -> Option<i32> {
        match name.to_lowercase().as_str() {
            "fast" => Some(1),
            "best" => Some(9),
            _ => None,
        }
    }
}

#[derive(Args, Debug)]
pub struct Bzip2Args {
    #[clap(flatten)]
    pub common_args: CommonArgs,

    #[clap(flatten)]
    pub progress_args: ProgressArgs,

    #[clap(flatten)]
    pub level_args: LevelArgs,
}

pub struct Bzip2 {
    pub level: i32, // 1-9
    pub progress_args: ProgressArgs,
}

impl Default for Bzip2 {
    fn default() -> Self {
        let validator = Bzip2CompressionValidator;
        Bzip2 {
            level: validator.default_level(),
            progress_args: ProgressArgs::default(),
        }
    }
}

impl Bzip2 {
    pub fn new(args: &Bzip2Args) -> Self {
        let validator = Bzip2CompressionValidator;
        let level = validator.validate_and_clamp_level(args.level_args.level.level);

        Bzip2 {
            level,
            progress_args: args.progress_args,
        }
    }
}

impl Compressor for Bzip2 {
    /// Default extension for bzip2 files
    fn extension(&self) -> &str {
        "bz2"
    }

    /// Name of this compressor
    fn name(&self) -> &str {
        "bzip2"
    }

    /// Bzip2 extracts to a file by default
    fn default_extracted_target(&self) -> ExtractedTarget {
        ExtractedTarget::FILE
    }

    /// Compress an input file or pipe to a bz2 archive
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
        let mut encoder = BzEncoder::new(output_stream, Compression::new(self.level as u32));

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

    /// Extract a bz2 archive to a file or pipe
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
        let mut decoder = BzDecoder::new(output_stream);

        // Use the custom output function to handle progress bar updates
        copy_with_progress(
            &mut input_stream,
            &mut decoder,
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

    /// Test the basic interface of the Bzip2 compressor
    #[test]
    fn test_bzip2_interface() {
        let compressor = Bzip2::default();
        test_compressor_interface(&compressor, "bzip2", Some("bz2"));
    }

    #[test]
    fn test_bzip2_compression_validator() {
        let validator = Bzip2CompressionValidator;
        test_compression_validator_helper(
            &validator,
            1,       // min_level
            9,       // max_level
            9,       // default_level
            Some(1), // fast_name_level
            Some(9), // best_name_level
            None,    // none_name_level
        );
    }

    /// Test the default compression level
    #[test]
    fn test_bzip2_default_compression() -> Result<(), io::Error> {
        let compressor = Bzip2::default();
        test_compression(&compressor)
    }

    /// Test fast compression level
    #[test]
    fn test_bzip2_fast_compression() -> Result<(), io::Error> {
        let fast_compressor = Bzip2 {
            level: 1,
            progress_args: ProgressArgs::default(),
        };
        test_compression(&fast_compressor)
    }

    /// Test best compression level
    #[test]
    fn test_bzip2_best_compression() -> Result<(), io::Error> {
        let best_compressor = Bzip2 {
            level: 9,
            progress_args: ProgressArgs::default(),
        };
        test_compression(&best_compressor)
    }
}
