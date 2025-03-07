use crate::progress::{copy_with_progress, ProgressArgs};
use crate::utils::{
    cmprss_error, CmprssInput, CmprssOutput, CommonArgs, CompressionLevelValidator, Compressor,
    LevelArgs,
};
use clap::Args;
use std::fs::File;
use std::io::{self, BufReader, BufWriter, Read, Write};
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

#[derive(Args, Debug)]
pub struct ZstdArgs {
    #[clap(flatten)]
    pub common_args: CommonArgs,

    #[clap(flatten)]
    pub level_args: LevelArgs,

    #[clap(flatten)]
    pub progress_args: ProgressArgs,
}

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

impl Zstd {
    pub fn new(args: &ZstdArgs) -> Zstd {
        let validator = ZstdCompressionValidator;
        let mut level = args.level_args.level.level;

        // Validate and clamp the level to zstd's valid range
        level = validator.validate_and_clamp_level(level);

        Zstd {
            compression_level: level,
            progress_args: args.progress_args,
        }
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

    /// Generate a default extracted filename
    /// zstd does not support extracting to a directory, so we return a default filename
    fn default_extracted_filename(&self, in_path: &std::path::Path) -> String {
        // If the file has no extension, return a default filename
        if in_path.extension().is_none() {
            return "archive".to_string();
        }
        // Otherwise, return the filename without the extension
        in_path.file_stem().unwrap().to_str().unwrap().to_string()
    }

    /// Compress an input file or pipe to a zstd archive
    fn compress(&self, input: CmprssInput, output: CmprssOutput) -> Result<(), io::Error> {
        if let CmprssOutput::Path(out_path) = &output {
            if out_path.is_dir() {
                return cmprss_error("Zstd does not support compressing to a directory. Please specify an output file.");
            }
        }
        if let CmprssInput::Path(input_paths) = &input {
            for x in input_paths {
                if x.is_dir() {
                    return cmprss_error(
                        "Zstd does not support compressing a directory. Please specify only files.",
                    );
                }
            }
        }
        let mut file_size = None;
        let mut input_stream: Box<dyn Read + Send> = match input {
            CmprssInput::Path(paths) => {
                if paths.len() > 1 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "Multiple input files not supported for zstd",
                    ));
                }
                let path = &paths[0];
                file_size = Some(std::fs::metadata(path)?.len());
                Box::new(BufReader::new(File::open(path)?))
            }
            CmprssInput::Pipe(stdin) => Box::new(BufReader::new(stdin)),
        };

        let output_stream: Box<dyn Write + Send> = match &output {
            CmprssOutput::Path(path) => Box::new(BufWriter::new(File::create(path)?)),
            CmprssOutput::Pipe(stdout) => Box::new(BufWriter::new(stdout)),
        };

        // Create a zstd encoder with the specified compression level
        let mut encoder = Encoder::new(output_stream, self.compression_level)?;

        // Copy the input to the encoder with progress reporting
        copy_with_progress(
            &mut input_stream,
            &mut encoder,
            self.progress_args.chunk_size.size_in_bytes,
            file_size,
            self.progress_args.progress,
            &output,
        )?;

        // Finish the encoder to ensure all data is written
        encoder.finish()?;

        Ok(())
    }

    /// Extract a zstd archive to an output file or pipe
    fn extract(&self, input: CmprssInput, output: CmprssOutput) -> Result<(), io::Error> {
        if let CmprssOutput::Path(out_path) = &output {
            if out_path.is_dir() {
                return cmprss_error("Zstd does not support extracting to a directory. Please specify an output file.");
            }
        }

        let input_stream: Box<dyn Read + Send> = match input {
            CmprssInput::Path(paths) => {
                if paths.len() > 1 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "Multiple input files not supported for zstd",
                    ));
                }
                let path = &paths[0];
                Box::new(BufReader::new(File::open(path)?))
            }
            CmprssInput::Pipe(stdin) => Box::new(BufReader::new(stdin)),
        };

        // Create a zstd decoder
        let mut decoder = Decoder::new(input_stream)?;

        let mut output_stream: Box<dyn Write + Send> = match &output {
            CmprssOutput::Path(path) => Box::new(BufWriter::new(File::create(path)?)),
            CmprssOutput::Pipe(stdout) => Box::new(BufWriter::new(stdout)),
        };

        // Copy the decoded data to the output with progress reporting
        copy_with_progress(
            &mut decoder,
            &mut output_stream,
            self.progress_args.chunk_size.size_in_bytes,
            None,
            self.progress_args.progress,
            &output,
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn roundtrip() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempdir()?;
        let input_path = dir.path().join("input.txt");
        let compressed_path = dir.path().join("input.txt.zst");
        let output_path = dir.path().join("output.txt");

        // Create a test file
        let test_data = b"Hello, world! This is a test file for zstd compression.";
        std::fs::write(&input_path, test_data)?;

        // Compress the file
        let zstd = Zstd::default();
        zstd.compress(
            CmprssInput::Path(vec![input_path.clone()]),
            CmprssOutput::Path(compressed_path.clone()),
        )?;

        // Extract the file
        zstd.extract(
            CmprssInput::Path(vec![compressed_path]),
            CmprssOutput::Path(output_path.clone()),
        )?;

        // Verify the contents
        let output_data = std::fs::read(output_path)?;
        assert_eq!(test_data.to_vec(), output_data);

        Ok(())
    }

    #[test]
    fn test_zstd_compression_validator() {
        let validator = ZstdCompressionValidator;

        // Test range
        assert_eq!(validator.min_level(), -7);
        assert_eq!(validator.max_level(), 22);
        assert_eq!(validator.default_level(), 1);

        // Test validation
        assert!(validator.is_valid_level(-7));
        assert!(validator.is_valid_level(0));
        assert!(validator.is_valid_level(22));
        assert!(!validator.is_valid_level(-8));
        assert!(!validator.is_valid_level(23));

        // Test clamping
        assert_eq!(validator.validate_and_clamp_level(-8), -7);
        assert_eq!(validator.validate_and_clamp_level(0), 0);
        assert_eq!(validator.validate_and_clamp_level(23), 22);

        // Test special names
        assert_eq!(validator.name_to_level("none"), Some(-7));
        assert_eq!(validator.name_to_level("fast"), Some(1));
        assert_eq!(validator.name_to_level("best"), Some(22));
        assert_eq!(validator.name_to_level("invalid"), None);
    }
}
