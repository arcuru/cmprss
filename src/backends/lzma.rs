use super::stream::{stream_compress, stream_extract};
use crate::{
    progress::ProgressArgs,
    utils::{
        CmprssInput, CmprssOutput, CommonArgs, CompressionLevelValidator, Compressor,
        DefaultCompressionValidator, LevelArgs, Result, StreamCodec, StreamWriter,
    },
};
use clap::Args;
use std::io::{self, Read, Write};
use xz2::read::XzDecoder;
use xz2::stream::{LzmaOptions, Stream};
use xz2::write::XzEncoder;

/// Memory limit passed to the LZMA decoder. `u64::MAX` disables the limit,
/// which matches the behavior of `xz --lzma1 -d` / `unlzma`.
const LZMA_DECODER_MEMLIMIT: u64 = u64::MAX;

#[derive(Args, Debug)]
pub struct LzmaArgs {
    #[clap(flatten)]
    pub common_args: CommonArgs,

    #[clap(flatten)]
    progress_args: ProgressArgs,

    #[clap(flatten)]
    pub level_args: LevelArgs,
}

#[derive(Clone)]
pub struct Lzma {
    pub level: i32,
    pub progress_args: ProgressArgs,
}

impl Default for Lzma {
    fn default() -> Self {
        let validator = DefaultCompressionValidator;
        Lzma {
            level: validator.default_level(),
            progress_args: ProgressArgs::default(),
        }
    }
}

impl Lzma {
    pub fn new(args: &LzmaArgs) -> Lzma {
        Lzma {
            level: args.level_args.resolve(&DefaultCompressionValidator),
            progress_args: args.progress_args,
        }
    }

    /// Build a fresh LZMA1 (`lzma_alone`) encoder stream at the configured level.
    fn encoder_stream(&self) -> Result<Stream> {
        let options = LzmaOptions::new_preset(self.level as u32)?;
        Ok(Stream::new_lzma_encoder(&options)?)
    }

    /// Build a fresh LZMA1 (`lzma_alone`) decoder stream.
    fn decoder_stream() -> Result<Stream> {
        Ok(Stream::new_lzma_decoder(LZMA_DECODER_MEMLIMIT)?)
    }
}

/// StreamCodec encoder for legacy LZMA1. Owns an `XzEncoder` driving an
/// `lzma_alone` stream; its `flush` is a no-op because LZMA1 rejects the
/// `LZMA_FULL_FLUSH` that `XzEncoder::flush` would issue. Finalization goes
/// through `try_finish` instead, mirroring the existing single-codec path.
struct LzmaStreamEncoder(XzEncoder<Box<dyn StreamWriter>>);

impl Write for LzmaStreamEncoder {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl StreamWriter for LzmaStreamEncoder {
    fn finish(self: Box<Self>) -> io::Result<()> {
        let mut encoder = (*self).0;
        encoder.try_finish()?;
        let inner = encoder.finish()?;
        inner.finish()
    }
}

impl StreamCodec for Lzma {
    fn encoder(&self, inner: Box<dyn StreamWriter>) -> io::Result<Box<dyn StreamWriter>> {
        let stream = self.encoder_stream().map_err(io::Error::other)?;
        Ok(Box::new(LzmaStreamEncoder(XzEncoder::new_stream(
            inner, stream,
        ))))
    }

    fn decoder(&self, inner: Box<dyn Read + Send>) -> io::Result<Box<dyn Read + Send>> {
        let stream = Self::decoder_stream().map_err(io::Error::other)?;
        Ok(Box::new(XzDecoder::new_stream(inner, stream)))
    }
}

impl Compressor for Lzma {
    /// The standard extension for legacy LZMA (`.lzma`) files.
    fn extension(&self) -> &str {
        "lzma"
    }

    /// Full name for lzma.
    fn name(&self) -> &str {
        "lzma"
    }

    fn as_stream_codec(&self) -> Option<&dyn StreamCodec> {
        Some(self)
    }

    fn progress_args(&self) -> Option<&ProgressArgs> {
        Some(&self.progress_args)
    }

    fn compress(&self, input: CmprssInput, output: CmprssOutput) -> Result {
        stream_compress(self, "LZMA", input, output, &self.progress_args)
    }

    fn extract(&self, input: CmprssInput, output: CmprssOutput) -> Result {
        stream_extract(self, "LZMA", input, output, &self.progress_args)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::*;

    /// Test the basic interface of the Lzma compressor
    #[test]
    fn test_lzma_interface() {
        let compressor = Lzma::default();
        test_compressor_interface(&compressor, "lzma", Some("lzma"));
    }

    /// Test the default compression level
    #[test]
    fn test_lzma_default_compression() -> Result {
        let compressor = Lzma::default();
        test_compression(&compressor)
    }

    /// Test fast compression level
    #[test]
    fn test_lzma_fast_compression() -> Result {
        let fast_compressor = Lzma {
            level: 1,
            progress_args: ProgressArgs::default(),
        };
        test_compression(&fast_compressor)
    }

    /// Test best compression level
    #[test]
    fn test_lzma_best_compression() -> Result {
        let best_compressor = Lzma {
            level: 9,
            progress_args: ProgressArgs::default(),
        };
        test_compression(&best_compressor)
    }
}
