use crate::progress::{copy_with_progress, ProgressArgs};
use crate::utils::*;
use clap::Args;
use flate2::write::GzEncoder;
use flate2::{read::GzDecoder, Compression};
use std::fs::File;
use std::io::{self, BufReader, BufWriter, Read, Write};

#[derive(Args, Debug)]
pub struct GzipArgs {
    #[clap(flatten)]
    pub common_args: CommonArgs,

    #[clap(flatten)]
    pub level_args: LevelArgs,

    #[clap(flatten)]
    pub progress_args: ProgressArgs,
}

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
        let validator = DefaultCompressionValidator;
        let level = args.level_args.level.level;

        // Validate and clamp the level to gzip's valid range
        let level = validator.validate_and_clamp_level(level);

        Gzip {
            compression_level: level,
            progress_args: args.progress_args,
        }
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

    /// Generate a default extracted filename
    /// gzip does not support extracting to a directory, so we return a default filename
    fn default_extracted_filename(&self, in_path: &std::path::Path) -> String {
        // If the file has no extension, return a default filename
        if in_path.extension().is_none() {
            return "archive".to_string();
        }
        // Otherwise, return the filename without the extension
        in_path.file_stem().unwrap().to_str().unwrap().to_string()
    }

    /// Compress an input file or pipe to a gzip archive
    fn compress(&self, input: CmprssInput, output: CmprssOutput) -> Result<(), io::Error> {
        if let CmprssOutput::Path(out_path) = &output {
            if out_path.is_dir() {
                return cmprss_error("Gzip does not support compressing to a directory. Please specify an output file.");
            }
        }
        if let CmprssInput::Path(input_paths) = &input {
            for x in input_paths {
                if x.is_dir() {
                    return cmprss_error(
                        "Gzip does not support compressing a directory. Please specify only files.",
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
                        "Multiple input files not supported for gzip",
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

        // Create a gzip encoder with the specified compression level
        let mut encoder = GzEncoder::new(
            output_stream,
            Compression::new(self.compression_level as u32),
        );

        // Use the custom output function to handle progress bar updates with CountingWriter
        copy_with_progress(
            &mut input_stream,
            &mut encoder,
            self.progress_args.chunk_size.size_in_bytes,
            file_size,
            self.progress_args.progress,
            &output,
        )?;

        encoder.finish()?;
        Ok(())
    }

    /// Extract a gzip archive
    fn extract(&self, input: CmprssInput, output: CmprssOutput) -> Result<(), io::Error> {
        let mut file_size = None;
        let input_stream: Box<dyn Read + Send> = match input {
            CmprssInput::Path(paths) => {
                if paths.len() > 1 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "Multiple input files not supported for gzip extraction",
                    ));
                }
                let path = &paths[0];
                file_size = Some(std::fs::metadata(path)?.len());
                Box::new(BufReader::new(File::open(path)?))
            }
            CmprssInput::Pipe(stdin) => Box::new(BufReader::new(stdin)),
        };

        let mut output_stream: Box<dyn Write + Send> = match &output {
            CmprssOutput::Path(path) => Box::new(BufWriter::new(File::create(path)?)),
            CmprssOutput::Pipe(stdout) => Box::new(BufWriter::new(stdout)),
        };

        let mut decoder = GzDecoder::new(input_stream);

        // Use the utility function to handle progress bar updates
        copy_with_progress(
            &mut decoder,
            &mut output_stream,
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
    fn roundtrip() -> Result<(), Box<dyn std::error::Error>> {
        let compressor = Gzip::default();

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
}
