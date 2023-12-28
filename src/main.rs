mod bzip2;
mod gzip;
mod progress;
mod tar;
mod utils;
mod xz;

use bzip2::{Bzip2, Bzip2Args};
use clap::{Parser, Subcommand};
use gzip::{Gzip, GzipArgs};
use is_terminal::IsTerminal;
use std::io;
use std::path::{Path, PathBuf};
use tar::{Tar, TarArgs};
use utils::*;
use xz::{Xz, XzArgs};

/// A compression multi-tool
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct CmprssArgs {
    /// Format
    #[command(subcommand)]
    format: Option<Format>,
}

#[derive(Subcommand, Debug)]
enum Format {
    /// tar archive format
    Tar(TarArgs),

    /// gzip compression
    #[clap(visible_alias = "gz")]
    Gzip(GzipArgs),

    /// xz compression
    Xz(XzArgs),

    /// bzip2 compression
    #[clap(visible_alias = "bz2")]
    Bzip2(Bzip2Args),
}

/// Get the input filename or return a default file
/// This file will be used to generate the output filename
fn get_input_filename(input: &CmprssInput) -> Result<&Path, io::Error> {
    match input {
        CmprssInput::Path(paths) => {
            if paths.is_empty() {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    "error: no input specified",
                ));
            }
            Ok(paths.first().unwrap())
        }
        CmprssInput::Pipe(_) => Ok(Path::new("archive")),
    }
}

#[derive(Debug)]
enum Action {
    Compress,
    Extract,
}

/// Defines a single compress/extract action to take.
#[derive(Debug)]
struct Job {
    input: CmprssInput,
    output: CmprssOutput,
    action: Action,
}

/// Parse the common args and determine the details of the job requested
fn get_job<T: Compressor>(compressor: &T, common_args: &CommonArgs) -> Result<Job, io::Error> {
    let action = {
        if common_args.compress {
            Action::Compress
        } else if common_args.extract || common_args.decompress {
            Action::Extract
        } else {
            Action::Compress
        }
    };

    let mut inputs = match &common_args.input {
        Some(input) => {
            let path = PathBuf::from(input);
            if !path.try_exists()? {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    "Specified input path does not exist",
                ));
            }
            vec![path]
        }
        None => Vec::new(),
    };
    let mut output = match &common_args.output {
        Some(output) => {
            let path = Path::new(output);
            if path.try_exists()? && !path.is_dir() {
                // Output path exists, bail out
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    "Specified output path already exists",
                ));
            }
            Some(path)
        }
        None => None,
    };

    // Process the io_list, check if there is an output first
    let mut io_list = common_args.io_list.clone();
    if output.is_none() {
        if let Some(possible_output) = common_args.io_list.last() {
            let path = Path::new(possible_output);
            if !path.try_exists()? {
                // Use the given path if it doesn't exist
                output = Some(path);
                io_list.pop();
            } else if path.is_dir() {
                match action {
                    Action::Compress => {
                        // A directory can potentially be a target output location or
                        // an input, for now assume it is an input.
                    }
                    Action::Extract => {
                        // Can extract to a directory, and it wouldn't make any sense as an input
                        output = Some(path);
                        io_list.pop();
                    }
                };
            } else {
                // TODO: append checks
            }
        }
    }
    // Validate the specified inputs
    // Everything in the io_list should be an input
    for input in &io_list {
        let path = PathBuf::from(input);
        if !path.try_exists()? {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Specified input path does not exist",
            ));
        }
        inputs.push(path);
    }

    // Fallback to stdin/stdout if we're missing files
    let cmprss_input = match inputs.is_empty() {
        true => {
            if !std::io::stdin().is_terminal()
                && !&common_args.ignore_pipes
                && !&common_args.ignore_stdin
            {
                CmprssInput::Pipe(std::io::stdin())
            } else {
                return Err(io::Error::new(io::ErrorKind::Other, "No specified input"));
            }
        }
        false => CmprssInput::Path(inputs),
    };
    let cmprss_output = match output {
        Some(path) => CmprssOutput::Path(path.to_path_buf()),
        None => {
            if !std::io::stdout().is_terminal()
                && !&common_args.ignore_pipes
                && !&common_args.ignore_stdout
            {
                CmprssOutput::Pipe(std::io::stdout())
            } else {
                match action {
                    Action::Compress => {
                        // Use a default filename
                        CmprssOutput::Path(PathBuf::from(
                            compressor
                                .default_compressed_filename(get_input_filename(&cmprss_input)?),
                        ))
                    }
                    Action::Extract => CmprssOutput::Path(PathBuf::from(
                        compressor.default_extracted_filename(get_input_filename(&cmprss_input)?),
                    )),
                }
            }
        }
    };

    Ok(Job {
        input: cmprss_input,
        output: cmprss_output,
        action,
    })
}

fn command<T: Compressor>(compressor: T, args: &CommonArgs) -> Result<(), io::Error> {
    let job = get_job(&compressor, args)?;

    // TODO: Print expected actions, and ask for confirmation if there's ambiguity
    match job.action {
        Action::Compress => compressor.compress(job.input, job.output)?,
        Action::Extract => compressor.extract(job.input, job.output)?,
    };

    Ok(())
}

fn main() {
    let args = CmprssArgs::parse();
    match args.format {
        Some(Format::Tar(a)) => command(Tar::new(&a), &a.common_args),
        Some(Format::Gzip(a)) => command(Gzip::new(&a), &a.common_args),
        Some(Format::Xz(a)) => command(Xz::new(&a), &a.common_args),
        Some(Format::Bzip2(a)) => command(Bzip2::new(&a), &a.common_args),
        _ => Err(io::Error::new(io::ErrorKind::Other, "unknown input")),
    }
    .unwrap_or_else(|e| {
        eprintln!("ERROR(cmprss): {}", e);
        std::process::exit(1);
    });
}
