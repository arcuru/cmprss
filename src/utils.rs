use std::io;
use std::path::{Path, PathBuf};

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
    #[allow(dead_code)]
    fn is_archive(&self, in_path: &Path) -> bool {
        in_path.extension().unwrap_or_default() == self.extension()
    }

    /// Generate the default name for the compressed file
    fn default_compressed_filename(&self, in_path: &Path) -> String {
        format!(
            "{}.{}",
            in_path.file_name().unwrap().to_str().unwrap(),
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
