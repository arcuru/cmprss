use super::stream::{copy_stream, guard_file_output, open_input, prepare_output};
use crate::progress::ProgressArgs;
use crate::utils::{
    CmprssInput, CmprssOutput, CommonArgs, CompressionLevelValidator, Compressor, LevelArgs, Result,
};
use clap::Args;
use zstd::stream::{read::Decoder, write::Encoder};

/// Zstd-specific compression validator (-7 to 22 range)
#[derive(Debug, Clone, Copy)]
pub struct ZstdCompressionValidator;

impl CompressionLevelValidator for ZstdCompressionValidator {
    fn min_level(&self) -> i32 {
        -7
    }
    fn max_level(&self) -> i32 {
        22
    }
    fn default_level(&self) -> i32 {
        1
    }

    fn name_to_level(&self, name: &str) -> Option<i32> {
        match name.to_lowercase().as_str() {
            "none" => Some(-7),
            "fast" => Some(1),
            "best" => Some(22),
            _ => None,
        }
    }
}

#[derive(Args, Debug)]
pub struct ZstdArgs {
    #[clap(flatten)]
    pub common_args: CommonArgs,

    #[clap(flatten)]
    pub level_args: LevelArgs,

    #[clap(flatten)]
    pub progress_args: ProgressArgs,
}

pub struct Zstd {
    pub compression_level: i32,
    pub progress_args: ProgressArgs,
}

impl Default for Zstd {
    fn default() -> Self {
        let validator = ZstdCompressionValidator;
        Zstd {
            compression_level: validator.default_level(),
            progress_args: ProgressArgs::default(),
        }
    }
}

impl Zstd {
    pub fn new(args: &ZstdArgs) -> Zstd {
        Zstd {
            compression_level: args.level_args.resolve(&ZstdCompressionValidator),
            progress_args: args.progress_args,
        }
    }
}

impl Compressor for Zstd {
    /// The standard extension for the zstd format.
    fn extension(&self) -> &str {
        "zst"
    }

    /// Full name for zstd.
    fn name(&self) -> &str {
        "zstd"
    }

    /// Compress an input file or pipe to a zstd archive
    fn compress(&self, input: CmprssInput, output: CmprssOutput) -> Result {
        guard_file_output(&output, "Zstd")?;
        let (input_stream, file_size) = open_input(input, "Zstd")?;
        let (writer, target) = prepare_output(output)?;
        let mut encoder = Encoder::new(writer, self.compression_level)?;
        copy_stream(
            input_stream,
            &mut encoder,
            file_size,
            &self.progress_args,
            target,
        )?;
        encoder.finish()?;
        Ok(())
    }

    /// Extract a zstd archive to an output file or pipe
    fn extract(&self, input: CmprssInput, output: CmprssOutput) -> Result {
        guard_file_output(&output, "Zstd")?;
        let (input_stream, file_size) = open_input(input, "Zstd")?;
        let decoder = Decoder::new(input_stream)?;
        let (writer, target) = prepare_output(output)?;
        copy_stream(decoder, writer, file_size, &self.progress_args, target)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::*;

    /// Test the basic interface of the Zstd compressor
    #[test]
    fn test_zstd_interface() {
        let compressor = Zstd::default();
        test_compressor_interface(&compressor, "zstd", Some("zst"));
    }

    /// Test the default compression level
    #[test]
    fn test_zstd_default_compression() -> Result {
        let compressor = Zstd::default();
        test_compression(&compressor)
    }

    /// Test fast compression level
    #[test]
    fn test_zstd_fast_compression() -> Result {
        let fast_compressor = Zstd {
            compression_level: 1,
            progress_args: ProgressArgs::default(),
        };
        test_compression(&fast_compressor)
    }

    /// Test best compression level
    #[test]
    fn test_zstd_best_compression() -> Result {
        let best_compressor = Zstd {
            compression_level: 22,
            progress_args: ProgressArgs::default(),
        };
        test_compression(&best_compressor)
    }

    #[test]
    fn test_zstd_compression_validator() {
        let validator = ZstdCompressionValidator;
        test_compression_validator_helper(
            &validator,
            -7,       // min_level
            22,       // max_level
            1,        // default_level
            Some(1),  // fast_name_level
            Some(22), // best_name_level
            Some(-7), // none_name_level
        );
    }
}
