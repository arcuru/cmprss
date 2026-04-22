use crate::progress::{ProgressArgs, copy_with_progress};
use crate::utils::*;
use anyhow::bail;
use brotli::{CompressorWriter, Decompressor};
use clap::Args;
use std::fs::File;
use std::io::{self, BufReader, BufWriter, Read, Write};

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
        let validator = BrotliCompressionValidator;
        let level = validator.validate_and_clamp_level(args.level_args.level.level);

        Brotli {
            compression_level: level,
            progress_args: args.progress_args,
        }
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

    /// Compress an input file or pipe to a brotli archive
    fn compress(&self, input: CmprssInput, output: CmprssOutput) -> Result {
        if let CmprssOutput::Path(out_path) = &output
            && out_path.is_dir()
        {
            bail!(
                "Brotli does not support compressing to a directory. Please specify an output file."
            );
        }
        if let CmprssInput::Path(input_paths) = &input {
            for x in input_paths {
                if x.is_dir() {
                    bail!(
                        "Brotli does not support compressing a directory. Please specify only files."
                    );
                }
            }
        }
        let mut file_size = None;
        let mut input_stream: Box<dyn Read + Send> = match input {
            CmprssInput::Path(paths) => {
                if paths.len() > 1 {
                    bail!("Multiple input files not supported for brotli");
                }
                let path = &paths[0];
                file_size = Some(std::fs::metadata(path)?.len());
                Box::new(BufReader::new(File::open(path)?))
            }
            CmprssInput::Pipe(stdin) => Box::new(BufReader::new(stdin)),
            CmprssInput::Reader(reader) => reader.0,
        };

        let quality = self.compression_level as u32;

        if let CmprssOutput::Writer(writer) = output {
            let mut encoder =
                CompressorWriter::new(writer, BROTLI_BUFFER_SIZE, quality, BROTLI_LGWIN);
            io::copy(&mut input_stream, &mut encoder)?;
            encoder.flush()?;
        } else {
            let output_stream: Box<dyn Write + Send> = match &output {
                CmprssOutput::Path(path) => Box::new(BufWriter::new(File::create(path)?)),
                CmprssOutput::Pipe(stdout) => Box::new(BufWriter::new(stdout)),
                CmprssOutput::Writer(_) => unreachable!(),
            };
            let mut encoder =
                CompressorWriter::new(output_stream, BROTLI_BUFFER_SIZE, quality, BROTLI_LGWIN);
            copy_with_progress(
                &mut input_stream,
                &mut encoder,
                self.progress_args.chunk_size.size_in_bytes,
                file_size,
                self.progress_args.progress,
                &output,
            )?;
            encoder.flush()?;
        }

        Ok(())
    }

    /// Extract a brotli archive to an output file or pipe
    fn extract(&self, input: CmprssInput, output: CmprssOutput) -> Result {
        if let CmprssOutput::Path(out_path) = &output
            && out_path.is_dir()
        {
            bail!(
                "Brotli does not support extracting to a directory. Please specify an output file."
            );
        }

        let mut file_size = None;
        let input_stream: Box<dyn Read + Send> = match input {
            CmprssInput::Path(paths) => {
                if paths.len() > 1 {
                    bail!("Multiple input files not supported for brotli extraction");
                }
                let path = &paths[0];
                file_size = Some(std::fs::metadata(path)?.len());
                Box::new(BufReader::new(File::open(path)?))
            }
            CmprssInput::Pipe(stdin) => Box::new(BufReader::new(stdin)),
            CmprssInput::Reader(reader) => reader.0,
        };

        let mut decoder = Decompressor::new(input_stream, BROTLI_BUFFER_SIZE);

        if let CmprssOutput::Writer(mut writer) = output {
            io::copy(&mut decoder, &mut writer)?;
        } else {
            let mut output_stream: Box<dyn Write + Send> = match &output {
                CmprssOutput::Path(path) => Box::new(BufWriter::new(File::create(path)?)),
                CmprssOutput::Pipe(stdout) => Box::new(BufWriter::new(stdout)),
                CmprssOutput::Writer(_) => unreachable!(),
            };
            copy_with_progress(
                &mut decoder,
                &mut output_stream,
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
