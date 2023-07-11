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

    /// Tar extraction needs to specify the directory, so use the current directory
    fn default_extracted_filename(&self, _in_path: &Path) -> String {
        ".".to_string()
    }

    fn compress(&self, input: CmprssInput, output: CmprssOutput) -> Result<(), io::Error> {
        match output {
            CmprssOutput::Pipe(pipe) => self.compress_internal(input, Builder::new(pipe)),
            CmprssOutput::Path(path) => {
                self.compress_internal(input, Builder::new(File::create(path)?))
            }
        }
    }

    fn extract(&self, input: CmprssInput, output: CmprssOutput) -> Result<(), io::Error> {
        match input {
            CmprssInput::Path(paths) => {
                if paths.len() > 1 {
                    return cmprss_error("only 1 archive can be extracted at a time");
                }
                self.extract_internal(Archive::new(File::open(paths[0])?), output)
            }
            CmprssInput::Pipe(pipe) => self.extract_internal(Archive::new(pipe), output),
        }
    }
}

impl Tar {
    /// Internal extract helper
    fn extract_internal<R: Read>(
        &self,
        mut archive: Archive<R>,
        output: CmprssOutput,
    ) -> Result<(), io::Error> {
        let out_path = match output {
            CmprssOutput::Pipe(_) => {
                return cmprss_error("error: tar does not support stdout as extract output")
            }
            CmprssOutput::Path(path) => path,
        };
        archive.unpack(out_path)
    }

    /// Internal compress helper
    fn compress_internal<W: Write>(
        &self,
        input: CmprssInput,
        mut archive: Builder<W>,
    ) -> Result<(), io::Error> {
        let input_files = match input {
            CmprssInput::Path(paths) => paths,
            CmprssInput::Pipe(_) => {
                return cmprss_error("error: tar does not support stdin as input")
            }
        };
        for in_file in input_files {
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
        }
        archive.finish()
    }
}
