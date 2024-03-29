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

#[derive(Debug, Clone, Copy)]
pub struct CompressionLevel {
    pub level: u32,
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
        if let Ok(level) = s.parse::<u32>() {
            if level < 10 {
                return Ok(CompressionLevel { level });
            } else {
                return Err("Compression level must be 0-9");
            }
        }
        let s = s.to_lowercase();
        match s.as_str() {
            "none" => Ok(CompressionLevel { level: 0 }),
            "fast" => Ok(CompressionLevel { level: 1 }),
            "best" => Ok(CompressionLevel { level: 9 }),
            _ => Err("Invalid compression level"),
        }
    }
}

#[derive(Args, Debug, Default, Clone, Copy)]
pub struct LevelArgs {
    /// Level of compression.
    /// This is an int 0-9, with 0 being no compression and 9 being highest compression.
    /// Also supports 'none', 'fast', and 'best'.
    #[arg(long, default_value = "6")]
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
    fn compression_level_parsing() {
        assert_eq!(CompressionLevel::from_str("0").unwrap().level, 0);
        assert_eq!(CompressionLevel::from_str("1").unwrap().level, 1);
        assert_eq!(CompressionLevel::from_str("9").unwrap().level, 9);
        assert_eq!(CompressionLevel::from_str("none").unwrap().level, 0);
        assert_eq!(CompressionLevel::from_str("fast").unwrap().level, 1);
        assert_eq!(CompressionLevel::from_str("best").unwrap().level, 9);
        assert!(CompressionLevel::from_str("-1").is_err());
        assert!(CompressionLevel::from_str("10").is_err());
        assert!(CompressionLevel::from_str("foo").is_err());
    }
}
