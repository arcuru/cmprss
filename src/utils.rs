use clap::Args;
use std::fmt;
use std::io;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::str::FromStr;

pub type Result<T = ()> = anyhow::Result<T>;

/// Enum to represent whether a compressor extracts to a file or directory by default
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtractedTarget {
    /// Extract to a single file (e.g., gzip, bzip2, xz)
    File,
    /// Extract to a directory (e.g., zip, tar)
    Directory,
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
    #[arg(short, long, visible_alias = "decompress")]
    pub extract: bool,

    /// Append the input(s) to an existing archive.
    /// Only supported by container formats that can grow in place (tar, zip);
    /// stream codecs and mixed pipelines like `tar.gz` will error.
    #[arg(short, long)]
    pub append: bool,

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

    /// Overwrite the output path if it already exists.
    #[arg(short, long)]
    pub force: bool,

    /// List the contents of an archive (for container formats like tar and zip).
    #[arg(short, long)]
    pub list: bool,
}

/// Trait for validating compression levels for different compressors
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
    #[cfg(test)]
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

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        // Check for an int
        if let Ok(level) = s.parse::<i32>() {
            return Ok(CompressionLevel { level });
        }

        // Otherwise expect a named level ("none"/"fast"/"best"). The concrete
        // compressor re-interprets this value through its own validator, so we
        // start from the default mapping.
        let level = DefaultCompressionValidator
            .name_to_level(&s.to_lowercase())
            .ok_or("Invalid compression level")?;
        Ok(CompressionLevel { level })
    }
}

#[derive(Args, Debug, Default, Clone, Copy)]
pub struct LevelArgs {
    /// Level of compression.
    /// `none`, `fast`, and `best` are mapped to appropriate values for each compressor.
    #[arg(long, default_value = "fast")]
    pub level: CompressionLevel,
}

impl LevelArgs {
    /// Resolve the user-requested compression level against a codec-specific
    /// validator, clamping to the validator's range. This is the standard way
    /// for a backend to turn `--level N` into a concrete integer it can pass
    /// to the underlying library.
    pub fn resolve(&self, validator: &impl CompressionLevelValidator) -> i32 {
        validator.validate_and_clamp_level(self.level.level)
    }
}

/// Produce an owned copy of a `Compressor` behind `Box<dyn Compressor>`,
/// preserving all configuration (compression level, progress args, pipeline
/// chain, etc). `Pipeline` uses this to hand owned instances to worker threads
/// without losing user-supplied settings.
///
/// Implementors don't write this manually — the blanket impl below covers any
/// `Compressor + Clone + 'static`. `Clone` itself can't be a supertrait of
/// `Compressor` because it would break object safety for `Box<dyn Compressor>`.
pub trait CompressorClone {
    fn clone_boxed(&self) -> Box<dyn Compressor>;
}

impl<T: Compressor + Clone + 'static> CompressorClone for T {
    fn clone_boxed(&self) -> Box<dyn Compressor> {
        Box::new(self.clone())
    }
}

/// Common interface for all compressor implementations
pub trait Compressor: CompressorClone + Send + Sync {
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
        ExtractedTarget::File
    }

    /// Detect if the input is an archive of this type
    /// Just checks the extension by default
    /// Some compressors may overwrite this to do more advanced detection
    fn is_archive(&self, in_path: &Path) -> bool {
        in_path
            .extension()
            .is_some_and(|ext| ext == self.extension())
    }

    /// Generate the default name for the compressed file
    fn default_compressed_filename(&self, in_path: &Path) -> String {
        let name = in_path
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or("archive");
        format!("{name}.{}", self.extension())
    }

    /// Generate the default extracted filename
    fn default_extracted_filename(&self, in_path: &Path) -> String {
        if self.default_extracted_target() == ExtractedTarget::Directory {
            return ".".to_string();
        }

        // If the file has no extension, return the current directory
        if let Some(ext) = in_path.extension() {
            // If the file has the extension for this type, return the filename without the extension
            if let Some(ext_str) = ext.to_str()
                && ext_str == self.extension()
                && let Some(stem) = in_path.file_stem()
                && let Some(stem_str) = stem.to_str()
            {
                return stem_str.to_string();
            }
        }
        "archive".to_string()
    }

    fn compress(&self, input: CmprssInput, output: CmprssOutput) -> Result;

    fn extract(&self, input: CmprssInput, output: CmprssOutput) -> Result;

    /// Append the input to an existing archive pointed at by `output`.
    ///
    /// The default implementation bails: only container formats that can grow
    /// in place — currently tar and zip — support appending. Stream codecs
    /// (gzip, xz, …) have no notion of entries, and compound pipelines like
    /// `tar.gz` would require decompress-then-recompress which defeats the
    /// point of an in-place append.
    fn append(&self, _input: CmprssInput, _output: CmprssOutput) -> Result {
        anyhow::bail!(
            "{} archives do not support --append; only container formats (tar, zip) can be appended to in place",
            self.name()
        )
    }

    /// List the contents of the archive to stdout.
    ///
    /// The default implementation bails: only container formats — `tar`,
    /// `zip`, and pipelines whose innermost layer is one of those — can
    /// meaningfully enumerate their contents. Stream codecs (gzip, xz, …)
    /// just compress a single byte stream and have nothing to list.
    fn list(&self, _input: CmprssInput) -> Result {
        anyhow::bail!(
            "{} archives cannot be listed; only container formats (tar, zip) support --list",
            self.name()
        )
    }
}

impl fmt::Debug for dyn Compressor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Compressor {{ name: {} }}", self.name())
    }
}

/// Wrapper for Read + Send to allow Debug
pub struct ReadWrapper(pub Box<dyn Read + Send>);

impl Read for ReadWrapper {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.read(buf)
    }
}

impl fmt::Debug for ReadWrapper {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ReadWrapper")
    }
}

/// Wrapper for Write + Send to allow Debug
pub struct WriteWrapper(pub Box<dyn Write + Send>);

impl Write for WriteWrapper {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}

impl fmt::Debug for WriteWrapper {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "WriteWrapper")
    }
}

/// Defines the possible inputs of a compressor
#[derive(Debug)]
pub enum CmprssInput {
    /// Path(s) to the input files.
    Path(Vec<PathBuf>),
    /// Input pipe
    Pipe(std::io::Stdin),
    /// In-memory reader (for piping between compressors)
    Reader(ReadWrapper),
}

/// Defines the possible outputs of a compressor
#[derive(Debug)]
pub enum CmprssOutput {
    Path(PathBuf),
    Pipe(std::io::Stdout),
    /// In-memory writer (for piping between compressors)
    Writer(WriteWrapper),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    /// A simple implementation of the Compressor trait for testing
    #[derive(Clone)]
    struct TestCompressor;

    impl Compressor for TestCompressor {
        fn name(&self) -> &str {
            "test"
        }

        // We'll use the default implementation for extension() and other methods

        fn compress(&self, _: CmprssInput, _: CmprssOutput) -> Result {
            Ok(())
        }

        fn extract(&self, _: CmprssInput, _: CmprssOutput) -> Result {
            Ok(())
        }
    }

    /// A compressor that overrides the default extension
    #[derive(Clone)]
    struct CustomExtensionCompressor;

    impl Compressor for CustomExtensionCompressor {
        fn name(&self) -> &str {
            "custom"
        }

        fn extension(&self) -> &str {
            "cst"
        }

        fn compress(&self, _: CmprssInput, _: CmprssOutput) -> Result {
            Ok(())
        }

        fn extract(&self, _: CmprssInput, _: CmprssOutput) -> Result {
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
