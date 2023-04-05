use std::io::{Read, Write};
use std::path::Path;

/// Trait for generic compression/extract over Read/Write objects
pub trait CmprssRead {
    fn compress<I: Read, O: Write>(&self, input: I, output: O);
    fn extract<I: Read, O: Write>(&self, input: I, output: O);
}

/// Trait for compressing/extracting from a file.
pub trait CmprssFile {
    /// Compress an input file
    fn compress_file<I: AsRef<Path>, O: Write>(&self, in_file: I, output: O);

    /// Extract a file
    fn extract_file<I: AsRef<Path>, O: Write>(&self, in_file: I, output: O);
}

pub struct CmprssCommonArgs {
    pub compress: bool,
    pub extract: bool,
    pub input: String,
    pub output: Option<String>,
}

/// Getter method for the common arguments
pub trait CmprssArgTrait {
    // TODO: There is probably a cleaner way to do this?
    fn common_args(&self) -> &CmprssCommonArgs;
}

/// Generic info about the given compressor.
pub trait CmprssInfo {
    fn extension(&self) -> &str;
    fn name(&self) -> &str;
}
