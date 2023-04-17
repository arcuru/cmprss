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

fn command_targets<T: Compressor>(compressor: T) -> Result<(), io::Error> {
    let args = compressor.common_args();
    // Input prefers stdin if that is a pipe, and falls back to reading from a file.
    let input = match std::io::stdin().is_terminal() {
        true => CmprssInput::Path(Path::new(&args.input)),
        false => CmprssInput::Pipe(std::io::stdin()),
    };
    let default_output = match args.extract {
        true => compressor.default_extracted_filename(Path::new(&args.input)),
        false => compressor.default_compressed_filename(Path::new(&args.input)),
    };
    // Output prefers the stdout if we're piping, and falls back to piping to a file.
    // TODO: Not sure that this output logic is the right thing to do
    // TODO: Properly handle the output file
    //  Fail/Warn on existence
    //  Remove if you've created a stub
    let output = match std::io::stdout().is_terminal() {
        false => CmprssOutput::Pipe(std::io::stdout()),
        true => {
            if args.output.is_none() {
                if !std::io::stdin().is_terminal() {
                    // Use the 'input' file as the output
                    // TODO: make input file optional and test existence
                    CmprssOutput::Path(Path::new(&args.input))
                } else {
                    CmprssOutput::Path(Path::new(&default_output))
                }
            } else {
                CmprssOutput::Path(Path::new(args.output.as_ref().unwrap()))
            }
        }
    };
    if args.compress {
        compressor.compress(input, output)?;
    } else if args.extract {
        compressor.extract(input, output)?;
    } else {
        // Neither compress or extract is specified.
        // Compress by default, warn if if looks like an archive.
        match &input {
            CmprssInput::Path(path) => {
                if path.extension().unwrap() == compressor.extension() {
                    return cmprss_error(
                &format!("error: input appears to already be a {} archive, exiting. Use '--compress' if needed.", compressor.name()));
                }
                compressor.compress(input, output)?;
            }
            _ => compressor.compress(input, output)?,
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
        Some(Format::Tar(a)) => command_targets(parse_tar(a)),
        //Some(Format::Extract(a)) => command_extract(a),
        Some(Format::Gzip(a)) => command_targets(parse_gzip(a)),
        _ => Err(io::Error::new(io::ErrorKind::Other, "unknown input")),
    }
}
