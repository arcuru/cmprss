pub mod backends;
pub mod progress;
pub mod test_utils;
pub mod utils;

use backends::*;
use clap::{Parser, Subcommand};
use is_terminal::IsTerminal;
use std::io;
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
        CmprssInput::Path(paths) => match paths.first() {
            Some(path) => Ok(path),
            None => Err(io::Error::new(
                io::ErrorKind::Other,
                "error: no input specified",
            )),
        },
        CmprssInput::Pipe(_) => Ok(Path::new("archive")),
        CmprssInput::Reader(_) => Ok(Path::new("piped_data")),
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
    // Prioritize checking for multi-level formats first
    if let Some(filename_str) = filename.to_str() {
        let parts: Vec<&str> = filename_str.split('.').collect();
        // A potential multi-level format like "archive.tar.gz" will have at least 3 parts
        if parts.len() >= 3 {
            // Get all available single compressors for matching extensions
            let single_compressors: Vec<Box<dyn Compressor>> = vec![
                Box::<Tar>::default(),
                Box::<Gzip>::default(),
                Box::<Xz>::default(),
                Box::<Bzip2>::default(),
                Box::<Zip>::default(),
                Box::<Zstd>::default(),
                Box::<Lz4>::default(),
            ];

            // Get extensions in reverse order (from right to left, e.g., "gz", then "tar")
            let mut extensions: Vec<String> = Vec::new();
            for i in 1..parts.len() {
                // Iterate from the last extension backwards
                // Stop before including the base filename part if it's just "filename.gz" (parts.len() would be 2)
                // This loop is for parts.len() >= 3, ensuring we look at actual extensions
                if parts.len() - i > 0 {
                    // Ensure we don't go out of bounds for the base filename part
                    extensions.push(parts[parts.len() - i].to_string());
                } else {
                    break; // Should not happen if parts.len() >=3 and i starts at 1
                }
            }

            let mut compressor_types: Vec<String> = Vec::new();
            for ext_part in &extensions {
                // e.g., ext_part is "gz", then "tar"
                let mut found_match = false;
                for sc in &single_compressors {
                    if sc.extension() == ext_part || sc.name() == ext_part {
                        compressor_types.push(sc.name().to_string());
                        found_match = true;
                        break;
                    }
                }
                if !found_match {
                    // If any extension part is not recognized, this is not a valid multi-level chain we know.
                    // Clear types and break, so we can fall back to simple single extension check.
                    compressor_types.clear();
                    break;
                }
            }

            // If we successfully identified a chain of known compressor types:
            // compressor_types would be e.g. ["gzip", "tar"] (outermost to innermost)
            if !compressor_types.is_empty() {
                // MultiLevelCompressor::from_names expects innermost to outermost.
                compressor_types.reverse(); // e.g., ["tar", "gzip"]
                return Some(create_multi_level_compressor(&compressor_types));
            }
            // If compressor_types is empty here, it means the multi-level parse failed (e.g. "file.foo.bar" with unknown foo/bar)
            // or an unknown extension was found in the chain. We'll fall through to single extension check.
        }
    }

    // Fallback: If not a recognized multi-level format, or if fewer than 3 parts (e.g. "file.gz"),
    // try matching a single known compressor extension.
    let single_compressors: Vec<Box<dyn Compressor>> = vec![
        Box::<Tar>::default(),
        Box::<Gzip>::default(),
        Box::<Xz>::default(),
        Box::<Bzip2>::default(),
        Box::<Zip>::default(),
        Box::<Zstd>::default(),
        Box::<Lz4>::default(),
    ];

    // Check if file extension matches any known format
    // This is now a fallback.
    // Ensure this doesn't misinterpret "foo.tar.gz" as just "gz" if multi-level check failed for some reason
    // A more robust check here might be to see if filename *only* ends with .ext and not .something_else.ext
    // For now, the standard check is:
    if let Some(filename_str) = filename.to_str() {
        for sc in single_compressors {
            // A simple "ends_with" can be problematic for "file.tar.gz" vs "file.gz"
            // We need to be more specific. The extension should be exactly the compressor's extension.
            let expected_extension = format!(".{}", sc.extension());
            if filename_str.ends_with(&expected_extension) {
                // Further check: ensure it's not something like ".tar.gz" being matched by ".gz"
                // if we want to be super sure, but the multi-level check should catch .tar.gz first.
                // A simple way: if it ends with ".tar.gz", Gzip (gz) should not match here IF Tar (tar) also exists.
                // The current structure relies on multi-level being caught first.
                // If multi-level parsing failed, then we check single extensions.
                // Example: "archive.gz" -> Gzip
                // Example: "archive.tar" -> Tar
                // Example: "archive.unknown.gz" -> Multi-level fails, then Gzip matches.
                return Some(sc);
            }
        }
    }
    None
}

/// Create a MultiLevelCompressor from a list of compressor types
fn create_multi_level_compressor(compressor_types: &[String]) -> Box<dyn Compressor> {
    // Create a MultiLevelCompressor from the list of compressor types
    match MultiLevelCompressor::from_names(compressor_types) {
        Ok(multi) => Box::new(multi),
        Err(_) => {
            // Fallback to the first compressor if there's an error
            match compressor_types[0].as_str() {
                "tar" => Box::<Tar>::default(),
                "gzip" | "gz" => Box::<Gzip>::default(),
                "xz" => Box::<Xz>::default(),
                "bzip2" | "bz2" => Box::<Bzip2>::default(),
                "zip" => Box::<Zip>::default(),
                "zstd" | "zst" => Box::<Zstd>::default(),
                "lz4" => Box::<Lz4>::default(),
                _ => Box::<Tar>::default(), // Default to tar if unknown
            }
        }
    }
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

        // Check if output is a directory - this is likely an extraction
        if output.is_dir() {
            // Try to determine compressor from the input file's extension(s)
            if let Some(input_path) = input.first() {
                if let Some(guessed_compressor) = get_compressor_from_filename(input_path) {
                    return (Some(guessed_compressor), Action::Extract);
                }
            }
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
                // Same format for input and output, can't decide
                if output.is_dir() {
                    // If output is a directory, we're probably extracting
                    return (Some(e), Action::Extract);
                }
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
            } else if output.is_dir() {
                // If output is a directory, we're probably extracting
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
                if let CmprssOutput::Path(path) = &cmprss_output {
                    compressor = get_compressor_from_filename(path);
                }
            }
            Action::Extract => {
                // Look at the input name
                if let CmprssInput::Path(paths) = &cmprss_input {
                    if paths.len() != 1 {
                        // When extracting, we expect a single input file
                        return Err(io::Error::new(
                            io::ErrorKind::Other,
                            "Expected a single archive to extract",
                        ));
                    }
                    compressor = get_compressor_from_filename(paths.first().unwrap());

                    // If we still couldn't guess the compressor, try harder with multi-level extraction
                    if compressor.is_none() && paths.len() == 1 {
                        if let Some(filename_str) = paths.first().unwrap().to_str() {
                            // Try to parse multi-level formats (e.g., tar.gz)
                            let parts: Vec<&str> = filename_str.split('.').collect();
                            if parts.len() >= 3 {
                                // Get all available compressors
                                let compressors: Vec<Box<dyn Compressor>> = vec![
                                    Box::<Tar>::default(),
                                    Box::<Gzip>::default(),
                                    Box::<Xz>::default(),
                                    Box::<Bzip2>::default(),
                                    Box::<Zip>::default(),
                                    Box::<Zstd>::default(),
                                    Box::<Lz4>::default(),
                                ];

                                // Get extensions in reverse order (from right to left)
                                let mut extensions: Vec<String> = Vec::new();
                                for i in 1..parts.len() {
                                    extensions.push(parts[parts.len() - i].to_string());
                                }

                                // Try to find a compressor for each extension
                                let mut compressor_types: Vec<String> = Vec::new();
                                for ext in &extensions {
                                    for compressor in &compressors {
                                        if compressor.extension() == ext || compressor.name() == ext
                                        {
                                            compressor_types.push(compressor.name().to_string());
                                            break;
                                        }
                                    }
                                }

                                // If we found compressor types, create a MultiLevelCompressor
                                if !compressor_types.is_empty() {
                                    compressor =
                                        Some(create_multi_level_compressor(&compressor_types));
                                }
                            }
                        }
                    }
                }
            }
            Action::Unknown => match (&cmprss_input, &cmprss_output) {
                (CmprssInput::Path(paths), CmprssOutput::Path(path)) => {
                    // Special case: if output is a directory, assume we're extracting
                    if path.is_dir() && paths.len() == 1 {
                        // For extraction to directory, try to determine compressor from input file
                        compressor = get_compressor_from_filename(paths.first().unwrap());
                        action = Action::Extract;

                        // If no compressor was found, try harder with multi-level detection
                        if compressor.is_none() {
                            if let Some(filename_str) = paths.first().unwrap().to_str() {
                                // Try to parse multi-level formats (e.g., tar.gz)
                                let parts: Vec<&str> = filename_str.split('.').collect();
                                if parts.len() >= 3 {
                                    // Get all available compressors
                                    let compressors: Vec<Box<dyn Compressor>> = vec![
                                        Box::<Tar>::default(),
                                        Box::<Gzip>::default(),
                                        Box::<Xz>::default(),
                                        Box::<Bzip2>::default(),
                                        Box::<Zip>::default(),
                                        Box::<Zstd>::default(),
                                        Box::<Lz4>::default(),
                                    ];

                                    // Get extensions in reverse order (from right to left)
                                    let mut extensions: Vec<String> = Vec::new();
                                    for i in 1..parts.len() {
                                        extensions.push(parts[parts.len() - i].to_string());
                                    }

                                    // Try to find a compressor for each extension
                                    let mut compressor_types: Vec<String> = Vec::new();
                                    for ext in &extensions {
                                        for compressor in &compressors {
                                            if compressor.extension() == ext
                                                || compressor.name() == ext
                                            {
                                                compressor_types
                                                    .push(compressor.name().to_string());
                                                break;
                                            }
                                        }
                                    }

                                    // If we found compressor types, create a MultiLevelCompressor
                                    if !compressor_types.is_empty() {
                                        compressor =
                                            Some(create_multi_level_compressor(&compressor_types));
                                    }
                                }
                            }

                            // If we still couldn't determine compressor, fail with a clear message
                            if compressor.is_none() {
                                return Err(io::Error::new(
                                    io::ErrorKind::Other,
                                    format!(
                                        "Couldn't determine how to extract {:?}",
                                        paths.first().unwrap()
                                    ),
                                ));
                            }
                        }
                    } else {
                        let (guessed_compressor, guessed_action) =
                            guess_from_filenames(paths, path, compressor);
                        compressor = guessed_compressor;
                        action = guessed_action;
                    }
                }
                (CmprssInput::Path(paths), CmprssOutput::Pipe(_)) => {
                    if compressor.is_none() {
                        if paths.len() != 1 {
                            return Err(io::Error::new(
                                io::ErrorKind::Other,
                                "Expected a single input file for piping to stdout",
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
                (CmprssInput::Pipe(_), CmprssOutput::Pipe(_)) => {
                    action = Action::Compress;
                }
                // Handle all Writer output cases
                (_, CmprssOutput::Writer(_)) => {
                    // Writer outputs are only supported in multi-level compression
                    // In main.rs we'll assume compression
                    action = Action::Compress;
                }
                // Handle all Reader input cases
                (&CmprssInput::Reader(_), _) => {
                    // For Reader input, we'll assume extraction
                    action = Action::Extract;
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
