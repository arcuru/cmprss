mod gzip;
mod tar;
mod utils;

use clap::{Args, Parser, Subcommand};
use std::fs::File;
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

/// Execute a tar command
fn command_tar(args: TarArgs) {
    let args = args.common_args;
    let input_path = Path::new(&args.input);
    if args.compress {
        let out = output_filename(input_path, &args.output, tar::EXT);
        tar::compress_file(input_path, File::create(out).unwrap());
    } else if args.extract {
        tar::extract_file(input_path, args.output.unwrap_or(".".to_string()));
    } else {
        // Neither is set.
        // Compress by default, warn if if looks like an archive.
        if input_path.extension().unwrap() == tar::EXT {
            println!(
                "error: input appears to already be a tar archive, exiting. Use '--compress' if needed."
            )
        } else {
            let out = output_filename(input_path, &args.output, tar::EXT);
            tar::compress_file(input_path, File::create(out).unwrap());
        }
    }
}

/// Execute an extract command.
///
/// Attempts to extract based on the file extension.
fn command_extract(args: ExtractArgs) {
    let input_path = Path::new(&args.input);
    match input_path.extension().unwrap().to_str().unwrap() {
        tar::EXT => tar::extract_file(input_path, args.output.unwrap_or(".".to_string())),
        _ => println!("error: unknown format "),
    }
}

/// Implement compression/extraction with a generic Compressor.
fn command_generic<T: CmprssArgTrait + CmprssRead + CmprssInfo>(compressor: T) {
    let args = compressor.common_args();
    let input_path = Path::new(&args.input);
    if args.compress {
        let out = output_filename(input_path, &args.output, compressor.extension());
        compressor.compress(File::open(input_path).unwrap(), File::create(out).unwrap());
    } else if args.extract {
        assert!(args.output.is_some(), "error: output filename required");
        compressor.extract(
            File::open(input_path).unwrap(),
            File::create(args.output.clone().unwrap()).unwrap(),
        );
    } else {
        // Neither is set.
        // Compress by default, warn if if looks like an archive.
        if input_path.extension().unwrap() == compressor.extension() {
            println!(
                "error: input appears to already be a {} archive, exiting. Use '--compress' if needed.", compressor.name()
            )
        } else {
            let out = output_filename(input_path, &args.output, compressor.extension());
            compressor.compress(File::open(input_path).unwrap(), File::create(out).unwrap());
        }
    }
}

fn parse_gzip(args: GzipArgs) -> gzip::Gzip {
    gzip::Gzip {
        compression_level: args.compression,
        common_args: args.common_args.into_common(),
    }
}

fn main() {
    let args = CmprssArgs::parse();
    match args.format {
        Some(Format::Tar(a)) => command_tar(a),
        Some(Format::Extract(a)) => command_extract(a),
        //Some(Format::Gzip(a)) => command_gzip(a),
        Some(Format::Gzip(a)) => command_generic(parse_gzip(a)),
        None => println!("none"),
    };
}
