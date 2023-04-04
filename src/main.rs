mod tar;

use clap::Parser;
use std::path::Path;

/// A compression multi-tool
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Input file
    #[arg(index = 1)]
    input: String,

    /// Output file/directory
    ///
    /// If it's not provided, the extension is inferred from the compression type.
    #[arg(index = 2)]
    output: Option<String>,

    /// Compress the input
    #[arg(short, long)]
    compress: bool,

    /// Extract the input
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

fn main() {
    let args = Args::parse();
    let input_path = Path::new(&args.input);
    if args.compress {
        let out = output_filename(input_path, args.output, tar::extension());
        tar::compress(input_path, out);
    } else if args.extract {
        tar::extract(input_path, args.output.unwrap_or(".".to_string()));
    } else {
        // Neither set, so infer based on filename
        if input_path.extension().unwrap() == tar::extension() {
            tar::extract(input_path, args.output.unwrap_or(".".to_string()));
        } else {
            let out = output_filename(input_path, args.output, tar::extension());
            tar::compress(input_path, out);
        }
    }
}
