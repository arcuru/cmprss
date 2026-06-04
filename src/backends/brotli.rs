use super::stream::{stream_compress, stream_extract};
use crate::progress::ProgressArgs;
use crate::utils::{
    CmprssInput, CmprssOutput, CommonArgs, CompressionLevelValidator, Compressor, LevelArgs,
    Result, StreamCodec, StreamWriter,
};
use brotli::{CompressorWriter, Decompressor};
use clap::Args;
use std::io::{self, Read, Write};

/// Brotli buffer size used when constructing the encoder/decoder.
const BROTLI_BUFFER_SIZE: usize = 4096;

/// Window size (log2) used by the Brotli encoder. 22 is the value used by the
/// reference implementation at quality >= 2 and fits data with no upper bound.
const BROTLI_LGWIN: u32 = 22;

/// Brotli-specific compression validator. Quality range is 0-11 per RFC 7932,
/// where 0 is fastest and 11 is maximum compression.
#[derive(Debug, Clone, Copy)]
pub struct BrotliCompressionValidator;

impl CompressionLevelValidator for BrotliCompressionValidator {
    fn min_level(&self) -> i32 {
        0
    }
    fn max_level(&self) -> i32 {
        11
    }
    fn default_level(&self) -> i32 {
        6
    }

    fn name_to_level(&self, name: &str) -> Option<i32> {
        match name.to_lowercase().as_str() {
            "none" => Some(0),
            "fast" => Some(1),
            "best" => Some(11),
            _ => None,
        }
    }
}

#[derive(Args, Debug)]
pub struct BrotliArgs {
    #[clap(flatten)]
    pub common_args: CommonArgs,

    #[clap(flatten)]
    pub level_args: LevelArgs,

    #[clap(flatten)]
    pub progress_args: ProgressArgs,
}

#[derive(Clone)]
pub struct Brotli {
    pub compression_level: i32,
    pub progress_args: ProgressArgs,
}

impl Default for Brotli {
    fn default() -> Self {
        let validator = BrotliCompressionValidator;
        Brotli {
            compression_level: validator.default_level(),
            progress_args: ProgressArgs::default(),
        }
    }
}

impl Brotli {
    pub fn new(args: &BrotliArgs) -> Brotli {
        Brotli {
            compression_level: args.level_args.resolve(&BrotliCompressionValidator),
            progress_args: args.progress_args,
        }
    }
}

struct BrotliStreamEncoder(CompressorWriter<Box<dyn StreamWriter>>);

impl Write for BrotliStreamEncoder {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}

impl StreamWriter for BrotliStreamEncoder {
    fn finish(self: Box<Self>) -> io::Result<()> {
        // CompressorWriter::into_inner runs BROTLI_OPERATION_FINISH (writing
        // the closing brotli frame to the inner writer) before returning it,
        // so this both finalizes brotli and lets us cascade-finish the inner.
        let inner = (*self).0.into_inner();
        inner.finish()
    }
}

impl StreamCodec for Brotli {
    fn encoder(&self, inner: Box<dyn StreamWriter>) -> io::Result<Box<dyn StreamWriter>> {
        Ok(Box::new(BrotliStreamEncoder(CompressorWriter::new(
            inner,
            BROTLI_BUFFER_SIZE,
            self.compression_level as u32,
            BROTLI_LGWIN,
        ))))
    }

    fn decoder(&self, inner: Box<dyn Read + Send>) -> io::Result<Box<dyn Read + Send>> {
        Ok(Box::new(Decompressor::new(inner, BROTLI_BUFFER_SIZE)))
    }
}

impl Compressor for Brotli {
    /// The standard extension for brotli-compressed files.
    fn extension(&self) -> &str {
        "br"
    }

    /// Full name for brotli.
    fn name(&self) -> &str {
        "brotli"
    }

    fn as_stream_codec(&self) -> Option<&dyn StreamCodec> {
        Some(self)
    }

    fn compress(&self, input: CmprssInput, output: CmprssOutput) -> Result {
        stream_compress(self, "Brotli", input, output, &self.progress_args)
    }

    fn extract(&self, input: CmprssInput, output: CmprssOutput) -> Result {
        stream_extract(self, "Brotli", input, output, &self.progress_args)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::*;

    /// Test the basic interface of the Brotli compressor
    #[test]
    fn test_brotli_interface() {
        let compressor = Brotli::default();
        test_compressor_interface(&compressor, "brotli", Some("br"));
    }

    /// Test the default compression level
    #[test]
    fn test_brotli_default_compression() -> Result {
        let compressor = Brotli::default();
        test_compression(&compressor)
    }

    /// Test fast compression level
    #[test]
    fn test_brotli_fast_compression() -> Result {
        let fast_compressor = Brotli {
            compression_level: 1,
            progress_args: ProgressArgs::default(),
        };
        test_compression(&fast_compressor)
    }

    /// Test best compression level
    #[test]
    fn test_brotli_best_compression() -> Result {
        let best_compressor = Brotli {
            compression_level: 11,
            progress_args: ProgressArgs::default(),
        };
        test_compression(&best_compressor)
    }

    #[test]
    fn test_brotli_compression_validator() {
        let validator = BrotliCompressionValidator;
        test_compression_validator_helper(
            &validator,
            0,        // min_level
            11,       // max_level
            6,        // default_level
            Some(1),  // fast_name_level
            Some(11), // best_name_level
            Some(0),  // none_name_level
        );
    }
}
