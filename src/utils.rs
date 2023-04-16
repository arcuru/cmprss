use std::fs::File;
use std::io::{self, Read, Write};
use std::path::Path;

pub struct CmprssCommonArgs {
    pub compress: bool,
    pub extract: bool,
    pub input: String,
    pub output: Option<String>,
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

    /// Generate the default name for the compressed file
    fn default_compressed_filename(&self, in_path: &Path) -> String {
        format!(
            "{}.{}",
            in_path.file_name().unwrap().to_str().unwrap(),
            self.extension()
        )
    }

    // Generate the default extracted filename
    fn default_extracted_filename(&self, in_path: &Path) -> String {
        in_path.file_stem().unwrap().to_str().unwrap().to_string()
    }

    /// Getter method for the common arguments
    // TODO: There is probably a cleaner way to do this?
    fn common_args(&self) -> &CmprssCommonArgs;

    /// Compress one path to another path
    fn compress_path_to_path<I: AsRef<Path>, O: AsRef<Path>>(
        &self,
        in_file: I,
        out_file: O,
    ) -> Result<(), io::Error> {
        self.compress_file(in_file, File::create(out_file)?)
    }

    /// Compress an input filename to a stream
    fn compress_file<I: AsRef<Path>, O: Write>(
        &self,
        in_file: I,
        output: O,
    ) -> Result<(), io::Error> {
        self.compress(File::open(in_file)?, output)
    }

    /// Extract one path to another path
    fn extract_path_to_path<I: AsRef<Path>, O: AsRef<Path>>(
        &self,
        in_file: I,
        out_file: O,
    ) -> Result<(), io::Error> {
        self.extract_file(in_file, File::create(out_file)?)
    }

    /// Extract an input filename to a stream
    fn extract_file<I: AsRef<Path>, O: Write>(
        &self,
        in_file: I,
        output: O,
    ) -> Result<(), io::Error> {
        self.extract(File::open(in_file)?, output)
    }

    /// Compress a Read trait object to a Write object.
    fn compress<I: Read, O: Write>(&self, input: I, output: O) -> Result<(), io::Error> {
        cmprss_error("compress unimplemented")
    }

    /// Extract a Read trait object to a Write object.
    fn extract<I: Read, O: Write>(&self, input: I, output: O) -> Result<(), io::Error> {
        cmprss_error("extract unimplemented")
    }

    /// Extract a Read trait object to a path.
    /// Some compressors require this instead of writing to a stream
    fn extract_to_path<I: Read, O: AsRef<Path>>(
        &self,
        input: I,
        out_path: O,
    ) -> Result<(), io::Error> {
        cmprss_error("extract_to_path unimplemented")
    }

    /// Extract a file to a path
    fn extract_file_to_path<I: AsRef<Path>, O: AsRef<Path>>(
        &self,
        input_file: I,
        out_directory: O,
    ) -> Result<(), io::Error> {
        let input_file = input_file.as_ref();
        let out_directory = out_directory.as_ref();
        self.extract_to_path(File::open(input_file)?, out_directory)
    }
}

fn cmprss_error(message: &str) -> Result<(), io::Error> {
    Err(io::Error::new(io::ErrorKind::Other, message))
}
