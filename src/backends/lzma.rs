use crate::{
    progress::{ProgressArgs, copy_with_progress},
    utils::*,
};
use anyhow::bail;
use clap::Args;
use std::{
    fs::File,
    io::{self, BufReader, BufWriter, Read, Write},
};
use xz2::read::XzDecoder;
use xz2::stream::{LzmaOptions, Stream};
use xz2::write::XzEncoder;

/// Memory limit passed to the LZMA decoder. `u64::MAX` disables the limit,
/// which matches the behavior of `xz --lzma1 -d` / `unlzma`.
const LZMA_DECODER_MEMLIMIT: u64 = u64::MAX;

/// Swallows `flush()` calls on the wrapped writer. The legacy LZMA1
/// (`lzma_alone`) encoder in liblzma rejects `LZMA_FULL_FLUSH`, which is what
/// the inner `XzEncoder::flush` issues, so progress/copy helpers that call
/// `flush` mid-stream must see a no-op flush and let `try_finish` (via Drop
/// or `finish()`) finalize the stream instead.
struct NoFlush<W>(W);

impl<W: Write> Write for NoFlush<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[derive(Args, Debug)]
pub struct LzmaArgs {
    #[clap(flatten)]
    pub common_args: CommonArgs,

    #[clap(flatten)]
    progress_args: ProgressArgs,

    #[clap(flatten)]
    pub level_args: LevelArgs,
}

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
        let validator = DefaultCompressionValidator;
        let level = validator.validate_and_clamp_level(args.level_args.level.level);

        Lzma {
            level,
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

impl Compressor for Lzma {
    /// The standard extension for legacy LZMA (`.lzma`) files.
    fn extension(&self) -> &str {
        "lzma"
    }

    /// Full name for lzma.
    fn name(&self) -> &str {
        "lzma"
    }

    fn compress(&self, input: CmprssInput, output: CmprssOutput) -> Result {
        if let CmprssOutput::Path(out_path) = &output
            && out_path.is_dir()
        {
            bail!(
                "LZMA does not support compressing to a directory. Please specify an output file."
            );
        }
        if let CmprssInput::Path(input_paths) = &input {
            for x in input_paths {
                if x.is_dir() {
                    bail!(
                        "LZMA does not support compressing a directory. Please specify only files."
                    );
                }
            }
        }

        let mut file_size = None;
        let mut input_stream: Box<dyn Read + Send> = match input {
            CmprssInput::Path(paths) => {
                if paths.len() > 1 {
                    bail!("Multiple input files not supported for lzma");
                }
                let path = &paths[0];
                file_size = Some(std::fs::metadata(path)?.len());
                Box::new(BufReader::new(File::open(path)?))
            }
            CmprssInput::Pipe(pipe) => Box::new(pipe) as Box<dyn Read + Send>,
            CmprssInput::Reader(reader) => reader.0,
        };

        if let CmprssOutput::Writer(writer) = output {
            let mut encoder = XzEncoder::new_stream(writer, self.encoder_stream()?);
            io::copy(&mut input_stream, &mut encoder)?;
            encoder.try_finish()?;
        } else {
            let output_stream: Box<dyn Write + Send> = match &output {
                CmprssOutput::Path(path) => Box::new(BufWriter::new(File::create(path)?)),
                CmprssOutput::Pipe(pipe) => Box::new(pipe) as Box<dyn Write + Send>,
                CmprssOutput::Writer(_) => unreachable!(),
            };
            let mut encoder = XzEncoder::new_stream(output_stream, self.encoder_stream()?);
            copy_with_progress(
                &mut input_stream,
                NoFlush(&mut encoder),
                self.progress_args.chunk_size.size_in_bytes,
                file_size,
                self.progress_args.progress,
                &output,
            )?;
            encoder.try_finish()?;
        }

        Ok(())
    }

    fn extract(&self, input: CmprssInput, output: CmprssOutput) -> Result {
        let mut file_size = None;
        let input_stream: Box<dyn Read + Send> = match input {
            CmprssInput::Path(paths) => {
                if paths.len() > 1 {
                    bail!("Multiple input files not supported for lzma extraction");
                }
                let path = &paths[0];
                file_size = Some(std::fs::metadata(path)?.len());
                Box::new(BufReader::new(File::open(path)?)) as Box<dyn Read + Send>
            }
            CmprssInput::Pipe(pipe) => Box::new(pipe) as Box<dyn Read + Send>,
            CmprssInput::Reader(reader) => reader.0,
        };
        let mut decoder = XzDecoder::new_stream(input_stream, Self::decoder_stream()?);

        if let CmprssOutput::Writer(mut writer) = output {
            io::copy(&mut decoder, &mut writer)?;
        } else {
            let mut output_stream: Box<dyn Write + Send> = match &output {
                CmprssOutput::Path(path) => Box::new(BufWriter::new(File::create(path)?)),
                CmprssOutput::Pipe(pipe) => Box::new(pipe) as Box<dyn Write + Send>,
                CmprssOutput::Writer(_) => unreachable!(),
            };
            copy_with_progress(
                &mut decoder,
                &mut *output_stream,
                self.progress_args.chunk_size.size_in_bytes,
                file_size,
                self.progress_args.progress,
                &output,
            )?;
        }

        Ok(())
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
