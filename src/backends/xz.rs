use super::stream::{copy_stream, guard_file_output, open_input, prepare_output};
use crate::{
    progress::ProgressArgs,
    utils::{
        CmprssInput, CmprssOutput, CommonArgs, CompressionLevelValidator, Compressor,
        DefaultCompressionValidator, LevelArgs, Result,
    },
};
use clap::Args;
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

#[derive(Clone)]
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
        Xz {
            level: args.level_args.resolve(&DefaultCompressionValidator),
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

    fn clone_boxed(&self) -> Box<dyn Compressor> {
        Box::new(self.clone())
    }

    fn compress(&self, input: CmprssInput, output: CmprssOutput) -> Result {
        guard_file_output(&output, "Xz")?;
        let (input_stream, file_size) = open_input(input, "Xz")?;
        let (writer, target) = prepare_output(output)?;
        let mut encoder = XzEncoder::new(writer, self.level as u32);
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

    fn extract(&self, input: CmprssInput, output: CmprssOutput) -> Result {
        guard_file_output(&output, "Xz")?;
        let (input_stream, file_size) = open_input(input, "Xz")?;
        let decoder = XzDecoder::new(input_stream);
        let (writer, target) = prepare_output(output)?;
        copy_stream(decoder, writer, file_size, &self.progress_args, target)?;
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
    fn test_xz_default_compression() -> Result {
        let compressor = Xz::default();
        test_compression(&compressor)
    }

    /// Test fast compression level
    #[test]
    fn test_xz_fast_compression() -> Result {
        let fast_compressor = Xz {
            level: 1,
            progress_args: ProgressArgs::default(),
        };
        test_compression(&fast_compressor)
    }

    /// Test best compression level
    #[test]
    fn test_xz_best_compression() -> Result {
        let best_compressor = Xz {
            level: 9,
            progress_args: ProgressArgs::default(),
        };
        test_compression(&best_compressor)
    }
}
