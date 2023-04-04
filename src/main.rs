mod tar;

use clap::{Args, Parser, Subcommand};
use std::fs::File;
use std::path::Path;

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
    /// Input file
    #[arg(index = 1)]
    input: String,

    /// Output file/directory
    ///
    /// If it's not provided, the extension is inferred from the compression type.
    #[arg(index = 2)]
    output: Option<String>,

    /// Compress the input (default)
    #[arg(short, long)]
    compress: bool,

    /// Extract the input file
    #[arg(short, long)]
    extract: bool,
}

/// Generates the output filename.
/// This either takes the given name or guesses the name based on the extension
fn output_filename(input: &Path, output: Option<String>, extension: &str) -> String {
    match output {
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
    let input_path = Path::new(&args.input);
    if args.compress {
        let out = output_filename(input_path, args.output, tar::EXT);
        tar::compress_file(input_path, File::create(out).unwrap());
    } else if args.extract {
        tar::extract_file(input_path, args.output.unwrap_or(".".to_string()));
    } else {
        // Neither is set.
        // Compress by default, warn if if looks like an archive.
        if input_path.extension().unwrap() == tar::EXT {
            println!(
                "error: input appears to be a tar archive, exiting. Use '--compress' if needed."
            )
        } else {
            let out = output_filename(input_path, args.output, tar::EXT);
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

fn main() {
    let args = CmprssArgs::parse();
    match args.format {
        Some(Format::Tar(a)) => command_tar(a),
        Some(Format::Extract(a)) => command_extract(a),
        None => println!("none"),
    };
}
