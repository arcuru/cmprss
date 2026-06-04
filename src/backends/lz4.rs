use super::stream::{stream_compress, stream_extract};
use crate::progress::ProgressArgs;
#[cfg(feature = "cli")]
use crate::utils::CommonArgs;
use crate::utils::{CmprssInput, CmprssOutput, Compressor, Result, StreamCodec, StreamWriter};
#[cfg(feature = "cli")]
use clap::Args;
use lz4_flex::frame::{FrameDecoder, FrameEncoder};
use std::io::{self, Read, Write};

#[cfg(feature = "cli")]
#[derive(Args, Debug)]
pub struct Lz4Args {
    #[clap(flatten)]
    pub common_args: CommonArgs,
}

#[derive(Default, Clone)]
pub struct Lz4 {
    pub progress_args: ProgressArgs,
}

#[cfg(feature = "cli")]
impl Lz4 {
    pub fn new(args: &Lz4Args) -> Lz4 {
        Lz4 {
            progress_args: args.common_args.progress_args,
        }
    }
}

struct Lz4StreamEncoder(FrameEncoder<Box<dyn StreamWriter>>);

impl Write for Lz4StreamEncoder {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}

impl StreamWriter for Lz4StreamEncoder {
    fn finish(self: Box<Self>) -> io::Result<()> {
        // FrameEncoder::finish returns lz4_flex::frame::Error, which converts
        // cleanly to io::Error.
        let inner = (*self).0.finish().map_err(io::Error::from)?;
        inner.finish()
    }
}

impl StreamCodec for Lz4 {
    fn encoder(&self, inner: Box<dyn StreamWriter>) -> io::Result<Box<dyn StreamWriter>> {
        Ok(Box::new(Lz4StreamEncoder(FrameEncoder::new(inner))))
    }

    fn decoder(&self, inner: Box<dyn Read + Send>) -> io::Result<Box<dyn Read + Send>> {
        Ok(Box::new(FrameDecoder::new(inner)))
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

    fn as_stream_codec(&self) -> Option<&dyn StreamCodec> {
        Some(self)
    }

    fn progress_args(&self) -> Option<&ProgressArgs> {
        Some(&self.progress_args)
    }

    fn compress(&self, input: CmprssInput, output: CmprssOutput) -> Result {
        stream_compress(self, "LZ4", input, output, &self.progress_args)
    }

    fn extract(&self, input: CmprssInput, output: CmprssOutput) -> Result {
        stream_extract(self, "LZ4", input, output, &self.progress_args)
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
    fn test_lz4_default_compression() -> Result {
        let compressor = Lz4::default();
        test_compression(&compressor)
    }
}
