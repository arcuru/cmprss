pub mod backends;
mod job;
pub mod progress;
#[cfg(test)]
pub mod test_utils;
pub mod utils;

use backends::*;
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::Shell;
use job::{Action, get_job};
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

    /// brotli compression
    #[clap(visible_alias = "br")]
    Brotli(BrotliArgs),

    /// snappy framed compression
    #[clap(visible_alias = "sz")]
    Snappy(SnappyArgs),

    /// lzma (legacy LZMA1) compression
    Lzma(LzmaArgs),

    /// Print a shell completion script to stdout.
    #[clap(hide = true)]
    Completions {
        /// Shell to generate completions for.
        #[arg(value_enum)]
        shell: Shell,
    },

    /// Print the cmprss(1) man page (in troff format) to stdout.
    #[clap(hide = true)]
    Manpage,
}

fn write_completions(shell: Shell) -> Result {
    let mut cmd = CmprssArgs::command();
    clap_complete::generate(shell, &mut cmd, "cmprss", &mut std::io::stdout());
    Ok(())
}

fn write_manpage() -> Result {
    let cmd = CmprssArgs::command();
    clap_mangen::Man::new(cmd).render(&mut std::io::stdout())?;
    Ok(())
}

fn command(compressor: Option<Box<dyn Compressor>>, args: &CommonArgs) -> Result {
    let job = get_job(compressor, args)?;
    match job.action {
        Action::Compress => job.compressor.compress(job.input, job.output),
        Action::Extract => job.compressor.extract(job.input, job.output),
        Action::List => job.compressor.list(job.input),
    }
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
        Some(Format::Brotli(a)) => command(Some(Box::new(Brotli::new(&a))), &a.common_args),
        Some(Format::Snappy(a)) => command(Some(Box::new(Snappy::new(&a))), &a.common_args),
        Some(Format::Lzma(a)) => command(Some(Box::new(Lzma::new(&a))), &a.common_args),
        Some(Format::Completions { shell }) => write_completions(shell),
        Some(Format::Manpage) => write_manpage(),
        None => command(None, &args.base_args),
    }
    .unwrap_or_else(|e| {
        eprintln!("ERROR(cmprss): {}", e);
        std::process::exit(1);
    });
}
