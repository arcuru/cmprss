use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::Shell;
use cmprss::job::{Action, get_job};
use cmprss::{CommonArgs, Compressor, Pipeline, Result, chain_from_format_str};

#[cfg(feature = "brotli")]
use cmprss::{Brotli, BrotliArgs};
#[cfg(feature = "bzip2")]
use cmprss::{Bzip2, Bzip2Args};
#[cfg(feature = "gzip")]
use cmprss::{Gzip, GzipArgs};
#[cfg(feature = "lz4")]
use cmprss::{Lz4, Lz4Args};
#[cfg(feature = "lzma")]
use cmprss::{Lzma, LzmaArgs};
#[cfg(feature = "sevenz")]
use cmprss::{SevenZ, SevenZArgs};
#[cfg(feature = "snappy")]
use cmprss::{Snappy, SnappyArgs};
#[cfg(feature = "tar")]
use cmprss::{Tar, TarArgs};
#[cfg(feature = "xz")]
use cmprss::{Xz, XzArgs};
#[cfg(feature = "zip")]
use cmprss::{Zip, ZipArgs};
#[cfg(feature = "zstd")]
use cmprss::{Zstd, ZstdArgs};

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
    #[cfg(feature = "tar")]
    Tar(TarArgs),

    /// gzip compression
    #[cfg(feature = "gzip")]
    #[clap(visible_alias = "gz")]
    Gzip(GzipArgs),

    /// xz compression
    #[cfg(feature = "xz")]
    Xz(XzArgs),

    /// bzip2 compression
    #[cfg(feature = "bzip2")]
    #[clap(visible_alias = "bz2")]
    Bzip2(Bzip2Args),

    /// zip archive format
    #[cfg(feature = "zip")]
    Zip(ZipArgs),

    /// zstd compression
    #[cfg(feature = "zstd")]
    #[clap(visible_alias = "zst")]
    Zstd(ZstdArgs),

    /// lz4 compression
    #[cfg(feature = "lz4")]
    Lz4(Lz4Args),

    /// brotli compression
    #[cfg(feature = "brotli")]
    #[clap(visible_alias = "br")]
    Brotli(BrotliArgs),

    /// snappy framed compression
    #[cfg(feature = "snappy")]
    #[clap(visible_alias = "sz")]
    Snappy(SnappyArgs),

    /// lzma (legacy LZMA1) compression
    #[cfg(feature = "lzma")]
    Lzma(LzmaArgs),

    /// 7-Zip archive format
    #[cfg(feature = "sevenz")]
    #[clap(name = "7z", visible_alias = "sevenz")]
    SevenZ(SevenZArgs),

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

/// If the first positional arg looks like a dotted format string (e.g.
/// `tar.gz`, `tgz`) and isn't an existing path on disk, remove it from
/// `io_list` and return the equivalent compressor chain. This gives the
/// compound formats the same ergonomic treatment as the `tar` subcommand
/// without cluttering `--help` with every permutation.
fn take_format_prefix(io_list: &mut Vec<String>) -> Option<Box<dyn Compressor>> {
    let first = io_list.first()?;
    if std::path::Path::new(first).exists() {
        return None;
    }
    let chain = chain_from_format_str(first)?;
    let format = first.clone();
    io_list.remove(0);
    Some(Box::new(Pipeline::with_format(chain, format)))
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
        Action::Append => job.compressor.append(job.input, job.output),
    }
}

fn main() {
    let args = CmprssArgs::parse();
    match args.format {
        #[cfg(feature = "tar")]
        Some(Format::Tar(a)) => command(Some(Box::new(Tar::new(&a))), &a.common_args),
        #[cfg(feature = "gzip")]
        Some(Format::Gzip(a)) => command(Some(Box::new(Gzip::new(&a))), &a.common_args),
        #[cfg(feature = "xz")]
        Some(Format::Xz(a)) => command(Some(Box::new(Xz::new(&a))), &a.common_args),
        #[cfg(feature = "bzip2")]
        Some(Format::Bzip2(a)) => command(Some(Box::new(Bzip2::new(&a))), &a.common_args),
        #[cfg(feature = "zip")]
        Some(Format::Zip(a)) => command(Some(Box::new(Zip::new(&a))), &a.common_args),
        #[cfg(feature = "zstd")]
        Some(Format::Zstd(a)) => command(Some(Box::new(Zstd::new(&a))), &a.common_args),
        #[cfg(feature = "lz4")]
        Some(Format::Lz4(a)) => command(Some(Box::new(Lz4::new(&a))), &a.common_args),
        #[cfg(feature = "brotli")]
        Some(Format::Brotli(a)) => command(Some(Box::new(Brotli::new(&a))), &a.common_args),
        #[cfg(feature = "snappy")]
        Some(Format::Snappy(a)) => command(Some(Box::new(Snappy::new(&a))), &a.common_args),
        #[cfg(feature = "lzma")]
        Some(Format::Lzma(a)) => command(Some(Box::new(Lzma::new(&a))), &a.common_args),
        #[cfg(feature = "sevenz")]
        Some(Format::SevenZ(a)) => command(Some(Box::new(SevenZ::new(&a))), &a.common_args),
        Some(Format::Completions { shell }) => write_completions(shell),
        Some(Format::Manpage) => write_manpage(),
        None => {
            let mut base_args = args.base_args;
            let compressor = take_format_prefix(&mut base_args.io_list);
            command(compressor, &base_args)
        }
    }
    .unwrap_or_else(|e| {
        eprintln!("ERROR(cmprss): {}", e);
        std::process::exit(1);
    });
}
