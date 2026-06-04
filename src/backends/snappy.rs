use super::stream::{stream_compress, stream_extract};
use crate::progress::ProgressArgs;
use crate::utils::{
    CmprssInput, CmprssOutput, CommonArgs, Compressor, Result, StreamCodec, StreamWriter,
};
use clap::Args;
use snap::read::FrameDecoder;
use snap::write::FrameEncoder;
use std::io::{self, Read, Write};

#[derive(Args, Debug)]
pub struct SnappyArgs {
    #[clap(flatten)]
    pub common_args: CommonArgs,

    #[clap(flatten)]
    pub progress_args: ProgressArgs,
}

#[derive(Default, Clone)]
pub struct Snappy {
    pub progress_args: ProgressArgs,
}

impl Snappy {
    pub fn new(args: &SnappyArgs) -> Snappy {
        Snappy {
            progress_args: args.progress_args,
        }
    }
}

struct SnappyStreamEncoder(FrameEncoder<Box<dyn StreamWriter>>);

impl Write for SnappyStreamEncoder {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}

impl StreamWriter for SnappyStreamEncoder {
    fn finish(self: Box<Self>) -> io::Result<()> {
        // FrameEncoder::into_inner flushes pending data; framed snappy has no
        // trailer beyond what flush writes, so this is the natural finalize.
        let inner = (*self).0.into_inner().map_err(|e| e.into_error())?;
        inner.finish()
    }
}

impl StreamCodec for Snappy {
    fn encoder(&self, inner: Box<dyn StreamWriter>) -> io::Result<Box<dyn StreamWriter>> {
        Ok(Box::new(SnappyStreamEncoder(FrameEncoder::new(inner))))
    }

    fn decoder(&self, inner: Box<dyn Read + Send>) -> io::Result<Box<dyn Read + Send>> {
        Ok(Box::new(FrameDecoder::new(inner)))
    }
}

impl Compressor for Snappy {
    /// The standard extension for framed snappy files, per Google's reference
    /// implementation.
    fn extension(&self) -> &str {
        "sz"
    }

    /// Full name for snappy.
    fn name(&self) -> &str {
        "snappy"
    }

    fn as_stream_codec(&self) -> Option<&dyn StreamCodec> {
        Some(self)
    }

    fn progress_args(&self) -> Option<&ProgressArgs> {
        Some(&self.progress_args)
    }

    fn compress(&self, input: CmprssInput, output: CmprssOutput) -> Result {
        stream_compress(self, "Snappy", input, output, &self.progress_args)
    }

    fn extract(&self, input: CmprssInput, output: CmprssOutput) -> Result {
        stream_extract(self, "Snappy", input, output, &self.progress_args)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::*;

    /// Test the basic interface of the Snappy compressor
    #[test]
    fn test_snappy_interface() {
        let compressor = Snappy::default();
        test_compressor_interface(&compressor, "snappy", Some("sz"));
    }

    /// Test that the round-trip produces identical data
    #[test]
    fn test_snappy_default_compression() -> Result {
        let compressor = Snappy::default();
        test_compression(&compressor)
    }
}
