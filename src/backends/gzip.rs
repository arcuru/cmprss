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

    /// Gzip extracts to a file by default
    fn default_extracted_target(&self) -> ExtractedTarget {
        ExtractedTarget::FILE
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
    use crate::test_utils::*;
    use std::fs;
    use std::io::{Read, Write};
    use tempfile::tempdir;

    /// Test the basic interface of the Gzip compressor
    #[test]
    fn test_gzip_interface() {
        let compressor = Gzip::default();
        test_compressor_interface(&compressor, "gzip", Some("gz"));
    }

    /// Test the default compression level
    #[test]
    fn test_gzip_default_compression() -> Result<(), io::Error> {
        let compressor = Gzip::default();
        test_compression(&compressor)
    }

    /// Test fast compression level
    #[test]
    fn test_gzip_fast_compression() -> Result<(), io::Error> {
        let fast_compressor = Gzip {
            compression_level: 1,
            progress_args: ProgressArgs::default(),
        };
        test_compression(&fast_compressor)
    }

    /// Test best compression level
    #[test]
    fn test_gzip_best_compression() -> Result<(), io::Error> {
        let best_compressor = Gzip {
            compression_level: 9,
            progress_args: ProgressArgs::default(),
        };
        test_compression(&best_compressor)
    }

    /// Test for gzip-specific behavior: handling of concatenated gzip archives
    #[test]
    fn test_concatenated_gzip() -> Result<(), io::Error> {
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
