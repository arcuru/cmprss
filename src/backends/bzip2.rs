use super::stream::{stream_compress, stream_extract};
use crate::{
    progress::ProgressArgs,
    utils::{
        CmprssInput, CmprssOutput, CommonArgs, CompressionLevelValidator, Compressor, LevelArgs,
        Result, StreamCodec, StreamWriter,
    },
};
use bzip2::Compression;
use bzip2::write::BzEncoder;
use clap::Args;
use std::io::{self, Read, Write};

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

struct Bzip2StreamEncoder(BzEncoder<Box<dyn StreamWriter>>);

impl Write for Bzip2StreamEncoder {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}

impl StreamWriter for Bzip2StreamEncoder {
    fn finish(self: Box<Self>) -> io::Result<()> {
        let inner = (*self).0.finish()?;
        inner.finish()
    }
}

impl StreamCodec for Bzip2 {
    fn encoder(&self, inner: Box<dyn StreamWriter>) -> io::Result<Box<dyn StreamWriter>> {
        Ok(Box::new(Bzip2StreamEncoder(BzEncoder::new(
            inner,
            Compression::new(self.level as u32),
        ))))
    }

    fn decoder(&self, inner: Box<dyn Read + Send>) -> io::Result<Box<dyn Read + Send>> {
        Ok(Box::new(bzip2::read::BzDecoder::new(inner)))
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

    fn as_stream_codec(&self) -> Option<&dyn StreamCodec> {
        Some(self)
    }

    fn progress_args(&self) -> Option<&ProgressArgs> {
        Some(&self.progress_args)
    }

    fn compress(&self, input: CmprssInput, output: CmprssOutput) -> Result {
        stream_compress(self, "Bzip2", input, output, &self.progress_args)
    }

    fn extract(&self, input: CmprssInput, output: CmprssOutput) -> Result {
        stream_extract(self, "Bzip2", input, output, &self.progress_args)
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
