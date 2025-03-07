use clap::Args;
use std::ffi::OsStr;
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};
use std::str::FromStr;

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
        // If the file has the extension for this type, return the filename without the extension
        if in_path.extension().unwrap() == self.extension() {
            return in_path.file_stem().unwrap().to_str().unwrap().to_string();
        }
        // If the file has no extension, return the current directory
        if in_path.extension().is_none() {
            return ".".to_string();
        }
        // Otherwise, return the current directory and hope for the best
        ".".to_string()
    }

    fn compress(&self, input: CmprssInput, output: CmprssOutput) -> Result<(), io::Error> {
        cmprss_error("compress_target unimplemented")
    }

    fn extract(&self, input: CmprssInput, output: CmprssOutput) -> Result<(), io::Error> {
        cmprss_error("extract_target unimplemented")
    }
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

    #[test]
    fn test_compression_level_parsing() {
        // Test numeric values
        assert_eq!(CompressionLevel::from_str("-7").unwrap().level, -7);
        assert_eq!(CompressionLevel::from_str("0").unwrap().level, 0);
        assert_eq!(CompressionLevel::from_str("1").unwrap().level, 1);
        assert_eq!(CompressionLevel::from_str("9").unwrap().level, 9);
        assert_eq!(CompressionLevel::from_str("22").unwrap().level, 22);

        // Test special names (these use DefaultCompressionValidator values)
        assert_eq!(CompressionLevel::from_str("none").unwrap().level, 0);
        assert_eq!(CompressionLevel::from_str("fast").unwrap().level, 1);
        assert_eq!(CompressionLevel::from_str("best").unwrap().level, 9);

        // Test invalid values
        assert!(CompressionLevel::from_str("foo").is_err());
    }

    #[test]
    fn test_default_compression_validator() {
        let validator = DefaultCompressionValidator;

        // Test range
        assert_eq!(validator.min_level(), 0);
        assert_eq!(validator.max_level(), 9);
        assert_eq!(validator.default_level(), 6);

        // Test validation
        assert!(validator.is_valid_level(0));
        assert!(validator.is_valid_level(5));
        assert!(validator.is_valid_level(9));
        assert!(!validator.is_valid_level(-1));
        assert!(!validator.is_valid_level(10));

        // Test clamping
        assert_eq!(validator.validate_and_clamp_level(-1), 0);
        assert_eq!(validator.validate_and_clamp_level(5), 5);
        assert_eq!(validator.validate_and_clamp_level(10), 9);

        // Test special names
        assert_eq!(validator.name_to_level("none"), Some(0));
        assert_eq!(validator.name_to_level("fast"), Some(1));
        assert_eq!(validator.name_to_level("best"), Some(9));
        assert_eq!(validator.name_to_level("invalid"), None);
    }
}
