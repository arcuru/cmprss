extern crate tar;

use std::fs::File;
use std::io::{self, Read, Write};
use std::path::Path;
use tar::{Archive, Builder};

use crate::utils::*;

pub struct Tar {
    pub common_args: CmprssCommonArgs,
}

impl Compressor for Tar {
    /// Full name for tar, also used for extension
    fn name(&self) -> &str {
        "tar"
    }

    fn common_args(&self) -> &CmprssCommonArgs {
        &self.common_args
    }

    /// Compress an input file or directory into a tar archive.
    fn compress_file<I: AsRef<Path>, O: Write>(
        &self,
        in_file: I,
        output: O,
    ) -> Result<(), io::Error> {
        let in_file = in_file.as_ref();
        println!("tar: Compressing {}", in_file.display());
        let mut archive = Builder::new(output);
        if in_file.is_file() {
            archive.append_file(in_file.file_name().unwrap(), &mut File::open(in_file)?)?;
        } else if in_file.is_dir() {
            archive.append_dir_all(in_file.file_name().unwrap(), in_file)?;
        } else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "unknown file type",
            ));
        }
        archive.finish()
    }

    /// Extract one path to another path
    fn extract_path_to_path<I: AsRef<Path>, O: AsRef<Path>>(
        &self,
        in_file: I,
        out_file: O,
    ) -> Result<(), io::Error> {
        self.extract_to_path(File::open(in_file)?, out_file)
    }

    /// Extract the archive into a directory
    fn extract_to_path<I: Read, O: AsRef<Path>>(
        &self,
        input: I,
        out_path: O,
    ) -> Result<(), io::Error> {
        println!("tar extracting to {}", out_path.as_ref().display());
        let mut archive = Archive::new(input);
        archive.unpack(out_path.as_ref())
    }
}
