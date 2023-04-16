mod gzip;
mod tar;
mod utils;

use clap::{Args, Parser, Subcommand};
use is_terminal::IsTerminal;
use std::io;
use std::path::Path;
use utils::*;

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

    /// extract by guessing the format
    Extract(ExtractArgs),

    /// gzip compression
    #[clap(visible_alias = "gz")]
    Gzip(GzipArgs),
}

#[derive(Args, Debug)]
struct ExtractArgs {
    /// Input file
    #[arg(index = 1)]
    input: String,

    /// Output file/directory
    #[arg(index = 2)]
    output: Option<String>,
}

#[derive(Args, Debug)]
struct TarArgs {
    #[clap(flatten)]
    common_args: CommonArgs,
}

#[derive(Args, Debug)]
struct CommonArgs {
    /// Input file
    #[arg(index = 1)]
    input: String,

    /// Output file/directory
    #[arg(index = 2)]
    output: Option<String>,

    /// Compress the input (default)
    #[arg(short, long)]
    compress: bool,

    /// Extract the input
    #[arg(short, long)]
    extract: bool,
}

impl CommonArgs {
    /// Convert clap argument struct into utils::CmprssCommonArgs
    /// This is done, perhaps unnecessarily, to keep clap out of the lib
    fn into_common(self) -> CmprssCommonArgs {
        CmprssCommonArgs {
            compress: self.compress,
            input: self.input,
            output: self.output,
            extract: self.extract,
        }
    }
}

#[derive(Args, Debug)]
struct GzipArgs {
    #[clap(flatten)]
    common_args: CommonArgs,

    /// Level of compression
    ///
    /// This is an int 0-9, with 0 being no compression and 9 being highest compression.
    #[arg(long, default_value_t = 6)]
    compression: u32,
}

/// Generates the output filename.
/// This either takes the given name or guesses the name based on the extension
fn output_filename(input: &Path, output: &Option<String>, extension: &str) -> String {
    match output.clone() {
        Some(file) => file,
        None => {
            format!(
                "{}.{}",
                input.file_name().unwrap().to_str().unwrap(),
                extension
            )
        }
    }
}

/// Compress using the compressor
fn compress_generic<T: Compressor>(compressor: T) -> Result<(), io::Error> {
    // TODO: Properly handle the output file
    //  Fail/Warn on existence
    //  Remove if you've created a stub
    let args = compressor.common_args();
    let input_path = Path::new(&args.input);

    match &args.output {
        Some(out) => {
            // Output file specified, use that
            println!("Compressing {} into {}", input_path.display(), out);
            compressor.compress_path_to_path(input_path, out)?;
        }
        None => {
            // No output filename. Send to stdout if stream or guess the filename
            if std::io::stdout().is_terminal() {
                let out = output_filename(input_path, &args.output, compressor.extension());
                println!("Compressing {} into {}", input_path.display(), out);
                compressor.compress_path_to_path(input_path, out)?;
            } else {
                // Stdout is a pipe, attempt to compress to that
                compressor.compress_file(input_path, std::io::stdout())?;
            }
        }
    }
    Ok(())
}

/// Implement compression/extraction with a generic Compressor.
fn command_generic<T: Compressor>(compressor: T) -> Result<(), io::Error> {
    let args = compressor.common_args();
    let input_path = Path::new(&args.input);
    if args.compress {
        compress_generic(compressor)?;
    } else if args.extract {
        match &args.output {
            Some(out) => {
                // Output file specified, extract there
                compressor.extract_path_to_path(input_path, out)?;
            }
            None => {
                // No output file specified
                if std::io::stdout().is_terminal() {
                    compressor.extract_path_to_path(
                        input_path,
                        compressor.default_extracted_filename(input_path),
                    )?;
                } else {
                    // Stdout is a pipe, extract to the pipe
                    compressor.extract_file(input_path, std::io::stdout())?;
                }
            }
        };
    } else {
        // Neither is set.
        // Compress by default, warn if if looks like an archive.
        if input_path.extension().unwrap() == compressor.extension() {
            println!(
                "error: input appears to already be a {} archive, exiting. Use '--compress' if needed.", compressor.name()
            )
        } else {
            compress_generic(compressor)?;
        }
    }
    Ok(())
}

fn parse_gzip(args: GzipArgs) -> gzip::Gzip {
    gzip::Gzip {
        compression_level: args.compression,
        common_args: args.common_args.into_common(),
    }
}

fn parse_tar(args: TarArgs) -> tar::Tar {
    tar::Tar {
        common_args: args.common_args.into_common(),
    }
}

fn main() -> Result<(), io::Error> {
    let args = CmprssArgs::parse();
    match args.format {
        Some(Format::Tar(a)) => command_generic(parse_tar(a)),
        //Some(Format::Extract(a)) => command_extract(a),
        Some(Format::Gzip(a)) => command_generic(parse_gzip(a)),
        _ => Err(io::Error::new(io::ErrorKind::Other, "unknown input")),
    }
}
