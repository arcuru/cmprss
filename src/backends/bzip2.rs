use crate::{
    progress::{copy_with_progress, ProgressArgs},
    utils::{
        cmprss_error, CmprssInput, CmprssOutput, CommonArgs, CompressionLevelValidator, Compressor,
        LevelArgs,
    },
};
use bzip2::write::{BzDecoder, BzEncoder};
use bzip2::Compression;
use clap::Args;
use std::{
    fs::File,
    io::{self, Read, Write},
};

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
        let validator = Bzip2CompressionValidator;
        let level = validator.validate_and_clamp_level(args.level_args.level.level);

        Bzip2 {
            level,
            progress_args: args.progress_args,
        }
    }
}

impl Compressor for Bzip2 {
    /// The standard extension for the bz2 format.
    fn extension(&self) -> &str {
        "bz2"
    }

    /// Full name for bz2.
    fn name(&self) -> &str {
        "bzip2"
    }

    /// Compress an input file or pipe to a bz2 archive
    fn compress(&self, input: CmprssInput, output: CmprssOutput) -> Result<(), io::Error> {
        let mut file_size = None;
        let mut input_stream = match input {
            CmprssInput::Path(paths) => {
                if paths.len() > 1 {
                    return cmprss_error("only 1 file can be compressed at a time");
                }
                let file = Box::new(File::open(paths[0].as_path())?);
                // Get the file size for the progress bar
                if let Ok(metadata) = file.metadata() {
                    file_size = Some(metadata.len());
                }
                file
            }
            CmprssInput::Pipe(pipe) => Box::new(pipe) as Box<dyn Read + Send>,
        };
        let output_stream: Box<dyn Write + Send> = match &output {
            CmprssOutput::Path(path) => Box::new(File::create(path)?),
            CmprssOutput::Pipe(pipe) => Box::new(pipe) as Box<dyn Write + Send>,
        };
        let mut encoder = BzEncoder::new(output_stream, Compression::new(self.level as u32));

        // Use the custom output function to handle progress bar updates
        copy_with_progress(
            &mut input_stream,
            &mut encoder,
            self.progress_args.chunk_size.size_in_bytes,
            file_size,
            self.progress_args.progress,
            &output,
        )?;

        Ok(())
    }

    /// Extract a bz2 archive to a file or pipe
    fn extract(&self, input: CmprssInput, output: CmprssOutput) -> Result<(), io::Error> {
        let mut file_size = None;
        let mut input_stream = match input {
            CmprssInput::Path(paths) => {
                if paths.len() > 1 {
                    return cmprss_error("only 1 file can be extracted at a time");
                }
                let file = Box::new(File::open(paths[0].as_path())?);
                // Get the file size for the progress bar
                if let Ok(metadata) = file.metadata() {
                    file_size = Some(metadata.len());
                }
                file
            }
            CmprssInput::Pipe(pipe) => Box::new(pipe) as Box<dyn Read + Send>,
        };
        let output_stream: Box<dyn Write + Send> = match &output {
            CmprssOutput::Path(path) => Box::new(File::create(path)?),
            CmprssOutput::Pipe(pipe) => Box::new(pipe) as Box<dyn Write + Send>,
        };
        let mut decoder = BzDecoder::new(output_stream);

        // Use the custom output function to handle progress bar updates
        copy_with_progress(
            &mut input_stream,
            &mut decoder,
            self.progress_args.chunk_size.size_in_bytes,
            file_size,
            self.progress_args.progress,
            &output,
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_fs::prelude::*;
    use predicates::prelude::*;

    #[test]
    fn test_bzip2_compression_validator() {
        let validator = Bzip2CompressionValidator;

        // Test range
        assert_eq!(validator.min_level(), 1);
        assert_eq!(validator.max_level(), 9);
        assert_eq!(validator.default_level(), 9);

        // Test validation
        assert!(validator.is_valid_level(1));
        assert!(validator.is_valid_level(5));
        assert!(validator.is_valid_level(9));
        assert!(!validator.is_valid_level(0));
        assert!(!validator.is_valid_level(10));

        // Test clamping
        assert_eq!(validator.validate_and_clamp_level(0), 1);
        assert_eq!(validator.validate_and_clamp_level(5), 5);
        assert_eq!(validator.validate_and_clamp_level(10), 9);

        // Test special names
        assert_eq!(validator.name_to_level("fast"), Some(1));
        assert_eq!(validator.name_to_level("best"), Some(9));
        assert_eq!(validator.name_to_level("none"), None);
        assert_eq!(validator.name_to_level("invalid"), None);
    }

    #[test]
    fn roundtrip() -> Result<(), Box<dyn std::error::Error>> {
        let compressor = Bzip2::default();

        let file = assert_fs::NamedTempFile::new("test.txt")?;
        file.write_str("garbage data for testing")?;
        let working_dir = assert_fs::TempDir::new()?;
        let archive = working_dir.child("archive.".to_owned() + compressor.extension());
        archive.assert(predicate::path::missing());

        // Roundtrip compress/extract
        compressor.compress(
            CmprssInput::Path(vec![file.path().to_path_buf()]),
            CmprssOutput::Path(archive.path().to_path_buf()),
        )?;
        archive.assert(predicate::path::is_file());
        compressor.extract(
            CmprssInput::Path(vec![archive.path().to_path_buf()]),
            CmprssOutput::Path(working_dir.child("test.txt").path().to_path_buf()),
        )?;

        // Assert the files are identical
        working_dir
            .child("test.txt")
            .assert(predicate::path::eq_file(file.path()));

        Ok(())
    }

    // Fail with a compression level of 0
    #[test]
    fn invalid_compression_level_0() {
        let compressor = Bzip2 {
            level: 0,
            ..Bzip2::default()
        };
        let file = assert_fs::NamedTempFile::new("test.txt").unwrap();
        let working_dir = assert_fs::TempDir::new().unwrap();
        let archive = working_dir.child("archive.".to_owned() + compressor.extension());
        let result = compressor.compress(
            CmprssInput::Path(vec![file.path().to_path_buf()]),
            CmprssOutput::Path(archive.path().to_path_buf()),
        );
        assert!(result.is_err());
    }

    // Fail with a compression level of 10
    #[test]
    fn invalid_compression_level_10() {
        let compressor = Bzip2 {
            level: 10,
            ..Bzip2::default()
        };
        let file = assert_fs::NamedTempFile::new("test.txt").unwrap();
        let working_dir = assert_fs::TempDir::new().unwrap();
        let archive = working_dir.child("archive.".to_owned() + compressor.extension());
        let result = compressor.compress(
            CmprssInput::Path(vec![file.path().to_path_buf()]),
            CmprssOutput::Path(archive.path().to_path_buf()),
        );
        assert!(result.is_err());
    }
}
