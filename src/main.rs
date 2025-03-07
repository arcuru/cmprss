mod backends;
mod progress;
mod utils;

use backends::*;
use clap::{Parser, Subcommand};
use is_terminal::IsTerminal;
use std::path::{Path, PathBuf};
use std::{io, vec};
use utils::*;

/// A compression multi-tool
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct CmprssArgs {
    /// Format
    #[command(subcommand)]
    format: Option<Format>,

    // Base arguments for the non-subcommand behavior
    #[clap(flatten)]
    pub base_args: CommonArgs,
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

    /// zip archive format
    Zip(ZipArgs),

    /// zstd compression
    #[clap(visible_alias = "zst")]
    Zstd(ZstdArgs),

    /// lz4 compression
    Lz4(Lz4Args),
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

#[derive(Debug, PartialEq, Clone, Copy)]
enum Action {
    Compress,
    Extract,
    Unknown,
}

/// Defines a single compress/extract action to take.
#[derive(Debug)]
struct Job {
    compressor: Box<dyn Compressor>,
    input: CmprssInput,
    output: CmprssOutput,
    action: Action,
}

/// Get a compressor from a filename
fn get_compressor_from_filename(filename: &Path) -> Option<Box<dyn Compressor>> {
    // TODO: Support multi-level files, like tar.gz
    let compressors: Vec<Box<dyn Compressor>> = vec![
        Box::<Tar>::default(),
        Box::<Gzip>::default(),
        Box::<Xz>::default(),
        Box::<Bzip2>::default(),
        Box::<Zip>::default(),
        Box::<Zstd>::default(),
        Box::<Lz4>::default(),
    ];
    compressors.into_iter().find(|c| c.is_archive(filename))
}

/// Convert an input path into a Path
fn get_path(input: &str) -> Option<PathBuf> {
    let path = PathBuf::from(input);
    if !path.try_exists().unwrap_or(false) {
        return None;
    }
    Some(path)
}

/// Guess compressor/action from the two filenames
/// The compressor may already be given
fn guess_from_filenames(
    input: &[PathBuf],
    output: &Path,
    compressor: Option<Box<dyn Compressor>>,
) -> (Option<Box<dyn Compressor>>, Action) {
    if input.len() != 1 {
        if let Some(guessed_compressor) = get_compressor_from_filename(output) {
            return (Some(guessed_compressor), Action::Compress);
        }
        // In theory we could be extracting multiple files to a directory
        // We'll fail somewhere else if that's not the case
        return (compressor, Action::Extract);
    }
    let input = input.first().unwrap();

    let guessed_compressor = get_compressor_from_filename(output);
    let guessed_extractor = get_compressor_from_filename(input);
    let guessed_compressor_name = if let Some(c) = &guessed_compressor {
        c.name()
    } else {
        ""
    };
    let guessed_extractor_name = if let Some(e) = &guessed_extractor {
        e.name()
    } else {
        ""
    };

    if let Some(c) = &compressor {
        if guessed_compressor_name == c.name() {
            return (compressor, Action::Compress);
        } else if guessed_extractor_name == c.name() {
            return (compressor, Action::Extract);
        } else {
            // Default to compressing
            return (compressor, Action::Compress);
        }
    }

    match (guessed_compressor, guessed_extractor) {
        (None, None) => (None, Action::Unknown),
        (Some(c), None) => (Some(c), Action::Compress),
        (None, Some(e)) => (Some(e), Action::Extract),
        (Some(c), Some(e)) => {
            if c.name() == e.name() {
                return (Some(c), Action::Unknown);
            }
            // Compare the input and output extensions to see if one has an extra extension
            let input_file = input.file_name().unwrap().to_str().unwrap();
            let input_ext = input.extension().unwrap_or_default();
            let output_file = output.file_name().unwrap().to_str().unwrap();
            let output_ext = output.extension().unwrap_or_default();
            let guessed_output = input_file.to_string() + "." + output_ext.to_str().unwrap();
            let guessed_input = output_file.to_string() + "." + input_ext.to_str().unwrap();
            if guessed_output == output_file {
                (Some(c), Action::Compress)
            } else if guessed_input == input_file {
                (Some(e), Action::Extract)
            } else {
                (None, Action::Unknown)
            }
        }
    }
}

/// Parse the common args and determine the details of the job requested
fn get_job(
    compressor: Option<Box<dyn Compressor>>,
    common_args: &CommonArgs,
) -> Result<Job, io::Error> {
    let mut compressor = compressor;
    let mut action = {
        if common_args.compress {
            Action::Compress
        } else if common_args.extract || common_args.decompress {
            Action::Extract
        } else {
            Action::Unknown
        }
    };

    let mut inputs = Vec::new();
    if let Some(in_file) = &common_args.input {
        match get_path(in_file) {
            Some(path) => inputs.push(path),
            None => {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    "Specified input path does not exist",
                ));
            }
        }
    }

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
                    _ => {
                        // TODO: don't know if this is an input or output, assume we're compressing this directory
                        // This does cause problems for inferencing "cat archive.tar | cmprss tar ."
                        // Probably need to add some special casing
                    }
                };
            } else {
                // TODO: check for scenarios where we want to append to an existing archive
            }
        }
    }

    // Validate the specified inputs
    // Everything in the io_list should be an input
    for input in &io_list {
        if let Some(path) = get_path(input) {
            inputs.push(path);
        } else {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Specified input path does not exist",
            ));
        }
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
                        if compressor.is_none() {
                            return Err(io::Error::new(
                                io::ErrorKind::Other,
                                "Must specify a compressor",
                            ));
                        }
                        CmprssOutput::Path(PathBuf::from(
                            compressor
                                .as_ref()
                                .unwrap()
                                .default_compressed_filename(get_input_filename(&cmprss_input)?),
                        ))
                    }
                    Action::Extract => {
                        if compressor.is_none() {
                            compressor =
                                get_compressor_from_filename(get_input_filename(&cmprss_input)?);
                            if compressor.is_none() {
                                return Err(io::Error::new(
                                    io::ErrorKind::Other,
                                    "Must specify a compressor",
                                ));
                            }
                        }
                        CmprssOutput::Path(PathBuf::from(
                            compressor
                                .as_ref()
                                .unwrap()
                                .default_extracted_filename(get_input_filename(&cmprss_input)?),
                        ))
                    }
                    Action::Unknown => {
                        if compressor.is_none() {
                            // Can still work if the input is an archive
                            compressor =
                                get_compressor_from_filename(get_input_filename(&cmprss_input)?);
                            if compressor.is_none() {
                                return Err(io::Error::new(
                                    io::ErrorKind::Other,
                                    "Must specify a compressor",
                                ));
                            }
                            action = Action::Extract;
                            CmprssOutput::Path(PathBuf::from(
                                compressor
                                    .as_ref()
                                    .unwrap()
                                    .default_extracted_filename(get_input_filename(&cmprss_input)?),
                            ))
                        } else {
                            // We know the compressor, does the input have the same extension?
                            if let Some(compressor_from_input) =
                                get_compressor_from_filename(get_input_filename(&cmprss_input)?)
                            {
                                if compressor.as_ref().unwrap().name()
                                    == compressor_from_input.name()
                                {
                                    action = Action::Extract;
                                    CmprssOutput::Path(PathBuf::from(
                                        compressor.as_ref().unwrap().default_extracted_filename(
                                            get_input_filename(&cmprss_input)?,
                                        ),
                                    ))
                                } else {
                                    action = Action::Compress;
                                    CmprssOutput::Path(PathBuf::from(
                                        compressor.as_ref().unwrap().default_compressed_filename(
                                            get_input_filename(&cmprss_input)?,
                                        ),
                                    ))
                                }
                            } else {
                                action = Action::Compress;
                                CmprssOutput::Path(PathBuf::from(
                                    compressor.as_ref().unwrap().default_compressed_filename(
                                        get_input_filename(&cmprss_input)?,
                                    ),
                                ))
                            }
                        }
                    }
                }
            }
        }
    };

    // If we don't have the compressor/action, we can attempt to infer
    if compressor.is_none() || action == Action::Unknown {
        match action {
            Action::Compress => {
                // Look at the output name
                // TODO: tar.gz ??
                if let CmprssOutput::Path(path) = &cmprss_output {
                    compressor = get_compressor_from_filename(path);
                }
            }
            Action::Extract => {
                // Look at the input name
                if let CmprssInput::Path(paths) = &cmprss_input {
                    if paths.len() != 1 {
                        // Can't guess if there are multiple inputs
                        return Err(io::Error::new(
                            io::ErrorKind::Other,
                            "Can't guess compressor with multiple inputs",
                        ));
                    }
                    compressor = get_compressor_from_filename(paths.first().unwrap());
                }
            }
            Action::Unknown => match (&cmprss_input, &cmprss_output) {
                (CmprssInput::Pipe(_), CmprssOutput::Path(path)) => {
                    if compressor.is_none() {
                        compressor = get_compressor_from_filename(path);
                        if compressor.is_some() {
                            action = Action::Compress;
                        } else {
                            return Err(io::Error::new(
                                io::ErrorKind::Other,
                                "Can't guess compressor to use",
                            ));
                        }
                    } else if compressor.as_ref().unwrap().name()
                        == get_compressor_from_filename(path).unwrap().name()
                    {
                        action = Action::Compress;
                    } else {
                        action = Action::Extract;
                    }
                }
                (CmprssInput::Path(paths), CmprssOutput::Pipe(_)) => {
                    if compressor.is_none() {
                        if paths.len() != 1 {
                            return Err(io::Error::new(
                                io::ErrorKind::Other,
                                "Can't guess compressor with multiple inputs",
                            ));
                        }
                        compressor = get_compressor_from_filename(paths.first().unwrap());
                        if compressor.is_some() {
                            action = Action::Extract;
                        } else {
                            return Err(io::Error::new(
                                io::ErrorKind::Other,
                                "Can't guess compressor to use",
                            ));
                        }
                    } else if let Some(c) = get_compressor_from_filename(paths.first().unwrap()) {
                        if compressor.as_ref().unwrap().name() == c.name() {
                            action = Action::Extract;
                        } else {
                            action = Action::Compress;
                        }
                    } else {
                        action = Action::Compress;
                    }
                }
                (CmprssInput::Pipe(_), CmprssOutput::Pipe(_)) => {
                    action = Action::Compress;
                }
                (CmprssInput::Path(paths), CmprssOutput::Path(path)) => {
                    let (guessed_compressor, guessed_action) =
                        guess_from_filenames(paths, path, compressor);
                    compressor = guessed_compressor;
                    action = guessed_action;
                }
            },
        }
    }

    if compressor.is_none() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "Could not determine compressor to use",
        ));
    }
    if action == Action::Unknown {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "Could not determine action to take",
        ));
    }

    Ok(Job {
        compressor: compressor.unwrap(),
        input: cmprss_input,
        output: cmprss_output,
        action,
    })
}

fn command(compressor: Option<Box<dyn Compressor>>, args: &CommonArgs) -> Result<(), io::Error> {
    let job = get_job(compressor, args)?;

    match job.action {
        Action::Compress => job.compressor.compress(job.input, job.output)?,
        Action::Extract => job.compressor.extract(job.input, job.output)?,
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Unknown action requested",
            ));
        }
    };

    Ok(())
}

fn main() {
    let args = CmprssArgs::parse();
    match args.format {
        Some(Format::Tar(a)) => command(Some(Box::new(Tar::new(&a))), &a.common_args),
        Some(Format::Gzip(a)) => command(Some(Box::new(Gzip::new(&a))), &a.common_args),
        Some(Format::Xz(a)) => command(Some(Box::new(Xz::new(&a))), &a.common_args),
        Some(Format::Bzip2(a)) => command(Some(Box::new(Bzip2::new(&a))), &a.common_args),
        Some(Format::Zip(a)) => command(Some(Box::new(Zip::new(&a))), &a.common_args),
        Some(Format::Zstd(a)) => command(Some(Box::new(Zstd::new(&a))), &a.common_args),
        Some(Format::Lz4(a)) => command(Some(Box::new(Lz4::new(&a))), &a.common_args),
        _ => command(None, &args.base_args),
    }
    .unwrap_or_else(|e| {
        eprintln!("ERROR(cmprss): {}", e);
        std::process::exit(1);
    });
}
