use super::stream::{stream_compress, stream_extract};
use crate::progress::ProgressArgs;
use crate::utils::{
    CmprssInput, CmprssOutput, CommonArgs, CompressionLevelValidator, Compressor,
    DefaultCompressionValidator, LevelArgs, Result, StreamCodec, StreamWriter,
};
use clap::Args;
use flate2::write::GzEncoder;
use flate2::{Compression, read::GzDecoder};
use std::io::{self, Read, Write};

#[derive(Args, Debug)]
pub struct GzipArgs {
    #[clap(flatten)]
    pub common_args: CommonArgs,

    #[clap(flatten)]
    pub level_args: LevelArgs,
}

#[derive(Clone)]
pub struct Gzip {
    pub compression_level: i32,
    pub progress_args: ProgressArgs,
}

impl Default for Gzip {
    fn default() -> Self {
        let validator = DefaultCompressionValidator;
        Gzip {
            compression_level: validator.default_level(),
            progress_args: ProgressArgs::default(),
        }
    }
}

impl Gzip {
    pub fn new(args: &GzipArgs) -> Gzip {
        Gzip {
            compression_level: args.level_args.resolve(&DefaultCompressionValidator),
            progress_args: args.common_args.progress_args,
        }
    }
}

/// Streaming encoder wrapper: a `GzEncoder` writing into the next
/// `StreamWriter` in the pipeline cascade. Boxed and returned from
/// `<Gzip as StreamCodec>::encoder`.
struct GzipStreamEncoder(GzEncoder<Box<dyn StreamWriter>>);

impl Write for GzipStreamEncoder {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}

impl StreamWriter for GzipStreamEncoder {
    fn finish(self: Box<Self>) -> io::Result<()> {
        // GzEncoder::finish flushes the gzip trailer and returns the inner
        // StreamWriter, which we then cascade-finalize.
        let inner = (*self).0.finish()?;
        inner.finish()
    }
}

impl StreamCodec for Gzip {
    fn encoder(&self, inner: Box<dyn StreamWriter>) -> io::Result<Box<dyn StreamWriter>> {
        Ok(Box::new(GzipStreamEncoder(GzEncoder::new(
            inner,
            Compression::new(self.compression_level as u32),
        ))))
    }

    fn decoder(&self, inner: Box<dyn Read + Send>) -> io::Result<Box<dyn Read + Send>> {
        Ok(Box::new(GzDecoder::new(inner)))
    }
}

impl Compressor for Gzip {
    /// The standard extension for the gzip format.
    fn extension(&self) -> &str {
        "gz"
    }

    /// Full name for gzip.
    fn name(&self) -> &str {
        "gzip"
    }

    fn as_stream_codec(&self) -> Option<&dyn StreamCodec> {
        Some(self)
    }

    fn progress_args(&self) -> Option<&ProgressArgs> {
        Some(&self.progress_args)
    }

    fn compress(&self, input: CmprssInput, output: CmprssOutput) -> Result {
        stream_compress(self, "Gzip", input, output, &self.progress_args)
    }

    fn extract(&self, input: CmprssInput, output: CmprssOutput) -> Result {
        stream_extract(self, "Gzip", input, output, &self.progress_args)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::*;
    use crate::utils::PassthroughWriter;
    use std::fs;
    use std::io::{Read, Write};
    use tempfile::tempdir;

    /// Test the basic interface of the Gzip compressor
    #[test]
    fn test_gzip_interface() {
        let compressor = Gzip::default();
        test_compressor_interface(&compressor, "gzip", Some("gz"));
    }

    /// Exercise the StreamCodec path end-to-end: wrap a captured sink with
    /// `encoder()`, write payload through the resulting `StreamWriter`, call
    /// `finish` to flush the gzip trailer (and cascade-finalize the
    /// passthrough), then round-trip the bytes back through `decoder()` and
    /// verify. This is the seam Pipeline will use in a later step, so it
    /// deserves a direct test independent of `compress`/`extract`.
    #[test]
    fn test_gzip_stream_codec_roundtrip() -> Result {
        use std::sync::{Arc, Mutex};
        // A Write that copies into a shared Vec, so we can read the encoded
        // bytes back after Box<dyn StreamWriter> has consumed the original.
        struct SharedSink(Arc<Mutex<Vec<u8>>>);
        impl Write for SharedSink {
            fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
                self.0.lock().unwrap().extend_from_slice(buf);
                Ok(buf.len())
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }

        let codec = Gzip::default();
        let payload = b"hello stream codec world".repeat(64);

        let captured = Arc::new(Mutex::new(Vec::new()));
        let passthrough: Box<dyn StreamWriter> =
            Box::new(PassthroughWriter(SharedSink(captured.clone())));
        let mut encoder = codec.encoder(passthrough)?;
        encoder.write_all(&payload)?;
        encoder.finish()?;

        let encoded = captured.lock().unwrap().clone();
        assert!(!encoded.is_empty(), "encoder produced no output");

        let cursor: Box<dyn Read + Send> = Box::new(std::io::Cursor::new(encoded));
        let mut decoder = codec.decoder(cursor)?;
        let mut decoded = Vec::new();
        decoder.read_to_end(&mut decoded)?;
        assert_eq!(decoded, payload);

        Ok(())
    }

    /// Test the default compression level
    #[test]
    fn test_gzip_default_compression() -> Result {
        let compressor = Gzip::default();
        test_compression(&compressor)
    }

    /// Test fast compression level
    #[test]
    fn test_gzip_fast_compression() -> Result {
        let fast_compressor = Gzip {
            compression_level: 1,
            progress_args: ProgressArgs::default(),
        };
        test_compression(&fast_compressor)
    }

    /// Test best compression level
    #[test]
    fn test_gzip_best_compression() -> Result {
        let best_compressor = Gzip {
            compression_level: 9,
            progress_args: ProgressArgs::default(),
        };
        test_compression(&best_compressor)
    }

    /// Test for gzip-specific behavior: handling of concatenated gzip archives
    #[test]
    fn test_concatenated_gzip() -> Result {
        let compressor = Gzip::default();
        let temp_dir = tempdir().expect("Failed to create temp dir");

        // Create two test files
        let input_path1 = temp_dir.path().join("input1.txt");
        let input_path2 = temp_dir.path().join("input2.txt");
        let test_data1 = "This is the first file";
        let test_data2 = "This is the second file";
        fs::write(&input_path1, test_data1)?;
        fs::write(&input_path2, test_data2)?;

        // Compress each file separately
        let archive_path1 = temp_dir.path().join("archive1.gz");
        let archive_path2 = temp_dir.path().join("archive2.gz");

        compressor.compress(
            CmprssInput::Path(vec![input_path1.clone()]),
            CmprssOutput::Path(archive_path1.clone()),
        )?;

        compressor.compress(
            CmprssInput::Path(vec![input_path2.clone()]),
            CmprssOutput::Path(archive_path2.clone()),
        )?;

        // Create a concatenated archive
        let concat_archive = temp_dir.path().join("concat.gz");
        let mut concat_file = fs::File::create(&concat_archive)?;

        // Concat the two gzip files
        let mut archive1_data = Vec::new();
        let mut archive2_data = Vec::new();
        fs::File::open(&archive_path1)?.read_to_end(&mut archive1_data)?;
        fs::File::open(&archive_path2)?.read_to_end(&mut archive2_data)?;

        concat_file.write_all(&archive1_data)?;
        concat_file.write_all(&archive2_data)?;
        concat_file.flush()?;

        // Extract the concatenated archive - this should yield the first file's contents
        let output_path = temp_dir.path().join("output.txt");

        compressor.extract(
            CmprssInput::Path(vec![concat_archive]),
            CmprssOutput::Path(output_path.clone()),
        )?;

        // Verify the result is the first file's content
        let output_data = fs::read_to_string(output_path)?;
        assert_eq!(output_data, test_data1);

        Ok(())
    }
}
