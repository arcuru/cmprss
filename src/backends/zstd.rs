use super::stream::{stream_compress, stream_extract};
use crate::progress::ProgressArgs;
use crate::utils::{
    CmprssInput, CmprssOutput, CompressionLevelValidator, Compressor, Result, StreamCodec,
    StreamWriter,
};
#[cfg(feature = "cli")]
use crate::utils::{CommonArgs, LevelArgs};
#[cfg(feature = "cli")]
use clap::Args;
use std::io::{self, Read, Write};
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

#[cfg(feature = "cli")]
#[derive(Args, Debug)]
pub struct ZstdArgs {
    #[clap(flatten)]
    pub common_args: CommonArgs,

    #[clap(flatten)]
    pub level_args: LevelArgs,
}

#[derive(Clone)]
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

#[cfg(feature = "cli")]
impl Zstd {
    pub fn new(args: &ZstdArgs) -> Zstd {
        Zstd {
            compression_level: args.level_args.resolve(&ZstdCompressionValidator),
            progress_args: args.common_args.progress_args,
        }
    }
}

struct ZstdStreamEncoder(Encoder<'static, Box<dyn StreamWriter>>);

impl Write for ZstdStreamEncoder {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}

impl StreamWriter for ZstdStreamEncoder {
    fn finish(self: Box<Self>) -> io::Result<()> {
        // zstd::stream::write::Encoder is one of the encoders that DOES lose
        // data on Drop — finish() is mandatory. It returns the inner writer
        // after writing the trailer.
        let inner = (*self).0.finish()?;
        inner.finish()
    }
}

impl StreamCodec for Zstd {
    fn encoder(&self, inner: Box<dyn StreamWriter>) -> io::Result<Box<dyn StreamWriter>> {
        Ok(Box::new(ZstdStreamEncoder(Encoder::new(
            inner,
            self.compression_level,
        )?)))
    }

    fn decoder(&self, inner: Box<dyn Read + Send>) -> io::Result<Box<dyn Read + Send>> {
        Ok(Box::new(Decoder::new(inner)?))
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

    fn as_stream_codec(&self) -> Option<&dyn StreamCodec> {
        Some(self)
    }

    fn progress_args(&self) -> Option<&ProgressArgs> {
        Some(&self.progress_args)
    }

    fn compress(&self, input: CmprssInput, output: CmprssOutput) -> Result {
        stream_compress(self, "Zstd", input, output, &self.progress_args)
    }

    fn extract(&self, input: CmprssInput, output: CmprssOutput) -> Result {
        stream_extract(self, "Zstd", input, output, &self.progress_args)
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
