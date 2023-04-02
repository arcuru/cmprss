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

    /// Output filename
    /// If it's not provided, the extension is inferred from the compression type.
    #[arg(index = 2)]
    output: Option<String>,
}

fn main() {
    let args = Args::parse();
    let out = match args.output {
        Some(file) => file,
        None => {
            // Append tar to the base file name as a default.
            let p = Path::new(&args.input);
            format!("{}{}", p.file_name().unwrap().to_str().unwrap(), ".tar")
        }
    };
    tar::compress(Path::new(&args.input), out);
}
