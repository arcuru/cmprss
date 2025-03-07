use clap::Args;
use std::ffi::OsStr;
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};
use std::str::FromStr;

/// Enum to represent whether a compressor extracts to a file or directory by default
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtractedTarget {
    /// Extract to a single file (e.g., gzip, bzip2, xz)
    FILE,
    /// Extract to a directory (e.g., zip, tar)
    DIRECTORY,
}

#[derive(Args, Debug)]
pub struct CommonArgs {
    /// Input file/directory
    #[arg(short, long)]
    pub input: Option<String>,

    /// Output file/directory
    #[arg(short, long)]
    pub output: Option<String>,

    /// Compress the input (default)
    #[arg(short, long)]
    pub compress: bool,

    /// Extract the input
    #[arg(short, long)]
    pub extract: bool,

    /// Decompress the input. Alias of --extract
    #[arg(short, long)]
    pub decompress: bool,

    /// List of I/O.
    /// This consists of all the inputs followed by the single output, with intelligent fallback to stdin/stdout.
    #[arg()]
    pub io_list: Vec<String>,

    /// Ignore pipes when inferring I/O
    #[arg(long)]
    pub ignore_pipes: bool,

    /// Ignore stdin when inferring I/O
    #[arg(long)]
    pub ignore_stdin: bool,

    /// Ignore stdout when inferring I/O
    #[arg(long)]
    pub ignore_stdout: bool,
}

/// Trait for validating compression levels for different compressors
#[allow(dead_code)]
pub trait CompressionLevelValidator {
    /// Get the minimum valid compression level
    fn min_level(&self) -> i32;

    /// Get the maximum valid compression level
    fn max_level(&self) -> i32;

    /// Get the default compression level
    fn default_level(&self) -> i32;

    /// Map special names to compression levels
    fn name_to_level(&self, name: &str) -> Option<i32>;

    /// Validate if a compression level is within the valid range
    fn is_valid_level(&self, level: i32) -> bool {
        level >= self.min_level() && level <= self.max_level()
    }

    /// Validate and clamp a compression level to the valid range
    fn validate_and_clamp_level(&self, level: i32) -> i32 {
        if level < self.min_level() {
            self.min_level()
        } else if level > self.max_level() {
            self.max_level()
        } else {
            level
        }
    }
}

/// Default implementation for most compressors (0-9 range)
#[derive(Debug, Clone, Copy)]
pub struct DefaultCompressionValidator;

impl CompressionLevelValidator for DefaultCompressionValidator {
    fn min_level(&self) -> i32 {
        0
    }
    fn max_level(&self) -> i32 {
        9
    }
    fn default_level(&self) -> i32 {
        6
    }

    fn name_to_level(&self, name: &str) -> Option<i32> {
        match name.to_lowercase().as_str() {
            "none" => Some(0),
            "fast" => Some(1),
            "best" => Some(9),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CompressionLevel {
    pub level: i32,
}

impl Default for CompressionLevel {
    fn default() -> Self {
        CompressionLevel { level: 6 }
    }
}

impl FromStr for CompressionLevel {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Check for an int
        if let Ok(level) = s.parse::<i32>() {
            return Ok(CompressionLevel { level });
        }

        // Try to parse special names
        let s = s.to_lowercase();
        match s.as_str() {
            "none" | "fast" | "best" => Ok(CompressionLevel {
                // We'll use the DefaultCompressionValidator values here
                // The actual compressor will interpret these values according to its own validator
                level: DefaultCompressionValidator.name_to_level(&s).unwrap(),
            }),
            _ => Err("Invalid compression level"),
        }
    }
}

#[derive(Args, Debug, Default, Clone, Copy)]
pub struct LevelArgs {
    /// Level of compression.
    /// `none`, `fast`, and `best` are mapped to appropriate values for each compressor.
    #[arg(long, default_value = "fast")]
    pub level: CompressionLevel,
}

/// Common interface for all compressor implementations
#[allow(unused_variables)]
pub trait Compressor {
    /// Name of this Compressor
    fn name(&self) -> &str;

    /// Default extension for this Compressor
    fn extension(&self) -> &str {
        self.name()
    }

    /// Determine if this compressor extracts to a file or directory by default
    /// FILE compressors (like gzip, bzip2, xz) extract to a single file
    /// DIRECTORY compressors (like zip, tar) extract to a directory
    fn default_extracted_target(&self) -> ExtractedTarget {
        ExtractedTarget::FILE
    }

    /// Detect if the input is an archive of this type
    /// Just checks the extension by default
    /// Some compressors may overwrite this to do more advanced detection
    fn is_archive(&self, in_path: &Path) -> bool {
        if in_path.extension().is_none() {
            return false;
        }
        in_path.extension().unwrap() == self.extension()
    }

    /// Generate the default name for the compressed file
    fn default_compressed_filename(&self, in_path: &Path) -> String {
        format!(
            "{}.{}",
            in_path
                .file_name()
                .unwrap_or_else(|| OsStr::new("archive"))
                .to_str()
                .unwrap(),
            self.extension()
        )
    }

    /// Generate the default extracted filename
    fn default_extracted_filename(&self, in_path: &Path) -> String {
        if self.default_extracted_target() == ExtractedTarget::DIRECTORY {
            return ".".to_string();
        }

        // If the file has no extension, return the current directory
        if let Some(ext) = in_path.extension() {
            // If the file has the extension for this type, return the filename without the extension
            if let Some(ext_str) = ext.to_str() {
                if ext_str == self.extension() {
                    if let Some(stem) = in_path.file_stem() {
                        if let Some(stem_str) = stem.to_str() {
                            return stem_str.to_string();
                        }
                    }
                }
            }
        }
        "archive".to_string()
    }

    fn compress(&self, input: CmprssInput, output: CmprssOutput) -> Result<(), io::Error>;

    fn extract(&self, input: CmprssInput, output: CmprssOutput) -> Result<(), io::Error>;
}

impl fmt::Debug for dyn Compressor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Compressor {{ name: {} }}", self.name())
    }
}

pub fn cmprss_error(message: &str) -> Result<(), io::Error> {
    Err(io::Error::new(io::ErrorKind::Other, message))
}

/// Defines the possible inputs of a compressor
#[derive(Debug)]
pub enum CmprssInput {
    /// Path(s) to the input files.
    Path(Vec<PathBuf>),
    /// Input pipe
    Pipe(std::io::Stdin),
}

/// Defines the possible outputs of a compressor
#[derive(Debug)]
pub enum CmprssOutput {
    Path(PathBuf),
    Pipe(std::io::Stdout),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;
    use std::path::Path;

    /// A simple implementation of the Compressor trait for testing
    struct TestCompressor;

    impl Compressor for TestCompressor {
        fn name(&self) -> &str {
            "test"
        }

        // We'll use the default implementation for extension() and other methods

        fn compress(&self, _: CmprssInput, _: CmprssOutput) -> Result<(), io::Error> {
            // Return success for testing purposes
            Ok(())
        }

        fn extract(&self, _: CmprssInput, _: CmprssOutput) -> Result<(), io::Error> {
            // Return success for testing purposes
            Ok(())
        }
    }

    /// A compressor that overrides the default extension
    struct CustomExtensionCompressor;

    impl Compressor for CustomExtensionCompressor {
        fn name(&self) -> &str {
            "custom"
        }

        fn extension(&self) -> &str {
            "cst"
        }

        fn compress(&self, _: CmprssInput, _: CmprssOutput) -> Result<(), io::Error> {
            Ok(())
        }

        fn extract(&self, _: CmprssInput, _: CmprssOutput) -> Result<(), io::Error> {
            Ok(())
        }
    }

    #[test]
    fn test_default_name_extension() {
        let compressor = TestCompressor;
        assert_eq!(compressor.name(), "test");
        assert_eq!(compressor.extension(), "test");
    }

    #[test]
    fn test_custom_extension() {
        let compressor = CustomExtensionCompressor;
        assert_eq!(compressor.name(), "custom");
        assert_eq!(compressor.extension(), "cst");
    }

    #[test]
    fn test_is_archive_detection() {
        use tempfile::tempdir;

        let compressor = TestCompressor;
        let temp_dir = tempdir().expect("Failed to create temp dir");

        // Test with matching extension
        let archive_path = temp_dir.path().join("archive.test");
        std::fs::File::create(&archive_path).expect("Failed to create test file");
        assert!(compressor.is_archive(&archive_path));

        // Test with non-matching extension
        let non_archive_path = temp_dir.path().join("archive.txt");
        std::fs::File::create(&non_archive_path).expect("Failed to create test file");
        assert!(!compressor.is_archive(&non_archive_path));

        // Test with no extension
        let no_ext_path = temp_dir.path().join("archive");
        std::fs::File::create(&no_ext_path).expect("Failed to create test file");
        assert!(!compressor.is_archive(&no_ext_path));
    }

    #[test]
    fn test_default_compressed_filename() {
        let compressor = TestCompressor;

        // Test with normal filename
        let path = Path::new("file.txt");
        assert_eq!(
            compressor.default_compressed_filename(path),
            "file.txt.test"
        );

        // Test with no extension
        let path = Path::new("file");
        assert_eq!(compressor.default_compressed_filename(path), "file.test");
    }

    #[test]
    fn test_default_extracted_filename() {
        let compressor = TestCompressor;

        // Test with matching extension
        let path = Path::new("archive.test");
        assert_eq!(compressor.default_extracted_filename(path), "archive");

        // Test with non-matching extension
        let path = Path::new("archive.txt");
        assert_eq!(compressor.default_extracted_filename(path), "archive");

        // Test with no extension
        let path = Path::new("archive");
        assert_eq!(compressor.default_extracted_filename(path), "archive");
    }

    #[test]
    fn test_compression_level_parsing() {
        // Test numeric levels
        assert_eq!(CompressionLevel::from_str("1").unwrap().level, 1);
        assert_eq!(CompressionLevel::from_str("9").unwrap().level, 9);

        // Test named levels
        let validator = DefaultCompressionValidator;
        assert_eq!(
            CompressionLevel::from_str("fast").unwrap().level,
            validator.name_to_level("fast").unwrap()
        );
        assert_eq!(
            CompressionLevel::from_str("best").unwrap().level,
            validator.name_to_level("best").unwrap()
        );

        // Test invalid values
        assert!(CompressionLevel::from_str("invalid").is_err());
    }

    #[test]
    fn test_compression_level_defaults() {
        let default_level = CompressionLevel::default();
        let validator = DefaultCompressionValidator;
        assert_eq!(default_level.level, validator.default_level());
    }

    #[test]
    fn test_cmprss_error() {
        let result = cmprss_error("test error");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "test error");
    }

    #[test]
    fn test_default_compression_validator() {
        let validator = DefaultCompressionValidator;

        use crate::test_utils::test_compression_validator_helper;
        test_compression_validator_helper(
            &validator,
            0,       // min_level
            9,       // max_level
            6,       // default_level
            Some(1), // fast_name_level
            Some(9), // best_name_level
            Some(0), // none_name_level
        );
    }
}
