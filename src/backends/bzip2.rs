use super::stream::{copy_stream, guard_file_output, open_input, prepare_output};
use crate::{
    progress::ProgressArgs,
    utils::{
        CmprssInput, CmprssOutput, CommonArgs, CompressionLevelValidator, Compressor, LevelArgs,
        Result,
    },
};
use bzip2::Compression;
use bzip2::write::{BzDecoder, BzEncoder};
use clap::Args;

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

#[derive(Clone)]
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
        Bzip2 {
            level: args.level_args.resolve(&Bzip2CompressionValidator),
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

    /// Compress an input file or pipe to a bz2 archive
    fn compress(&self, input: CmprssInput, output: CmprssOutput) -> Result {
        guard_file_output(&output, "Bzip2")?;
        let (input_stream, file_size) = open_input(input, "Bzip2")?;
        let (writer, target) = prepare_output(output)?;
        let mut encoder = BzEncoder::new(writer, Compression::new(self.level as u32));
        copy_stream(
            input_stream,
            &mut encoder,
            file_size,
            &self.progress_args,
            target,
        )?;
        Ok(())
    }

    /// Extract a bz2 archive to a file or pipe. Unlike most decoders,
    /// `BzDecoder` is write-driven: it wraps the output writer and we feed
    /// compressed bytes into it.
    fn extract(&self, input: CmprssInput, output: CmprssOutput) -> Result {
        guard_file_output(&output, "Bzip2")?;
        let (input_stream, file_size) = open_input(input, "Bzip2")?;
        let (writer, target) = prepare_output(output)?;
        let mut decoder = BzDecoder::new(writer);
        copy_stream(
            input_stream,
            &mut decoder,
            file_size,
            &self.progress_args,
            target,
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
    fn test_bzip2_default_compression() -> Result {
        let compressor = Bzip2::default();
        test_compression(&compressor)
    }

    /// Test fast compression level
    #[test]
    fn test_bzip2_fast_compression() -> Result {
        let fast_compressor = Bzip2 {
            level: 1,
            progress_args: ProgressArgs::default(),
        };
        test_compression(&fast_compressor)
    }

    /// Test best compression level
    #[test]
    fn test_bzip2_best_compression() -> Result {
        let best_compressor = Bzip2 {
            level: 9,
            progress_args: ProgressArgs::default(),
        };
        test_compression(&best_compressor)
    }
}
