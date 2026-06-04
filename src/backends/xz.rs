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
use xz2::write::XzEncoder;

#[derive(Args, Debug)]
pub struct XzArgs {
    #[clap(flatten)]
    pub common_args: CommonArgs,

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
            progress_args: args.common_args.progress_args,
        }
    }
}

struct XzStreamEncoder(XzEncoder<Box<dyn StreamWriter>>);

impl Write for XzStreamEncoder {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}

impl StreamWriter for XzStreamEncoder {
    fn finish(self: Box<Self>) -> io::Result<()> {
        let inner = (*self).0.finish()?;
        inner.finish()
    }
}

impl StreamCodec for Xz {
    fn encoder(&self, inner: Box<dyn StreamWriter>) -> io::Result<Box<dyn StreamWriter>> {
        Ok(Box::new(XzStreamEncoder(XzEncoder::new(
            inner,
            self.level as u32,
        ))))
    }

    fn decoder(&self, inner: Box<dyn Read + Send>) -> io::Result<Box<dyn Read + Send>> {
        Ok(Box::new(XzDecoder::new(inner)))
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

    fn as_stream_codec(&self) -> Option<&dyn StreamCodec> {
        Some(self)
    }

    fn progress_args(&self) -> Option<&ProgressArgs> {
        Some(&self.progress_args)
    }

    fn compress(&self, input: CmprssInput, output: CmprssOutput) -> Result {
        stream_compress(self, "Xz", input, output, &self.progress_args)
    }

    fn extract(&self, input: CmprssInput, output: CmprssOutput) -> Result {
        stream_extract(self, "Xz", input, output, &self.progress_args)
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
