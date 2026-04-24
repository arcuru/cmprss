//! Job inference — maps user-provided CLI args and filenames into a concrete
//! `Compressor` + action + input/output triple.
//!
//! Most of the user-facing ergonomics of `cmprss` live here: guessing whether
//! we're compressing or extracting, whether the input is piped from stdin,
//! which compressor a filename implies, and how to dispatch compound extensions
//! like `.tar.gz` or `.tgz` into a pipeline.

use anyhow::{anyhow, bail};
use is_terminal::IsTerminal;
use std::path::{Path, PathBuf};

use crate::backends::{self, Pipeline};
use crate::utils::{CmprssInput, CmprssOutput, CommonArgs, Compressor, Result};

/// Extract an action hint from the CLI flags. Returns `None` when the user
/// hasn't specified `--compress`/`--extract`/`--append`, in which case the
/// action will be inferred from filenames downstream.
fn action_from_flags(args: &CommonArgs) -> Option<Action> {
    if args.compress {
        Some(Action::Compress)
    } else if args.extract {
        Some(Action::Extract)
    } else if args.append {
        Some(Action::Append)
    } else {
        None
    }
}

/// Partition the CLI path arguments (`-i`, `-o`, and the positional `io_list`)
/// into a list of input paths and an optional output path.
///
/// The heuristic for which trailing `io_list` entry becomes the output:
/// * If it doesn't exist on disk → output (we'll create it).
/// * If it exists and is a directory, and the action hint is `Extract` →
///   output (extract into the directory).
/// * Otherwise → treat as an input. This preserves the existing behavior for
///   `cmprss tar file1.txt file2.txt existing_dir/`, where the trailing
///   directory is treated as another input to archive.
fn partition_paths(
    args: &CommonArgs,
    action_hint: Option<Action>,
) -> Result<(Vec<PathBuf>, Option<PathBuf>)> {
    let mut inputs = Vec::new();
    if let Some(in_file) = &args.input {
        inputs
            .push(get_path(in_file).ok_or_else(|| anyhow!("Specified input path does not exist"))?);
    }

    let mut output: Option<PathBuf> = match &args.output {
        Some(output) => {
            let path = PathBuf::from(output);
            if !args.force && path.try_exists()? && !path.is_dir() {
                bail!("Specified output path already exists (use --force to overwrite)");
            }
            Some(path)
        }
        None => None,
    };

    let mut io_list = args.io_list.clone();
    if output.is_none()
        && let Some(possible_output) = io_list.last()
    {
        let path = PathBuf::from(possible_output);
        if !path.try_exists()? {
            output = Some(path);
            io_list.pop();
        } else if path.is_dir() && action_hint == Some(Action::Extract) {
            // Only treat an existing directory as the output when the user
            // hinted extraction. In Compress/Unknown, we keep it as another
            // input — this matches e.g. `cmprss tar dir1/ dir2/`.
            output = Some(path);
            io_list.pop();
        } else if !path.is_dir() && args.force {
            // With --force, a trailing existing file is taken as the output
            // (to overwrite). Without --force we fall through to treating it
            // as another input.
            output = Some(path);
            io_list.pop();
        } else if !path.is_dir() && action_hint == Some(Action::Append) {
            // --append takes the trailing existing file as the archive to
            // grow. Same positional convention as compress: the target is
            // last, the new inputs are before it.
            output = Some(path);
            io_list.pop();
        }
    }

    for input in &io_list {
        inputs.push(get_path(input).ok_or_else(|| anyhow!("Specified input path does not exist"))?);
    }

    Ok((inputs, output))
}

/// Turn the collected input paths into a `CmprssInput`, falling back to
/// stdin when no paths were given and stdin is piped.
fn resolve_input(inputs: Vec<PathBuf>, args: &CommonArgs) -> Result<CmprssInput> {
    if !inputs.is_empty() {
        return Ok(CmprssInput::Path(inputs));
    }
    if !std::io::stdin().is_terminal() && !args.ignore_pipes && !args.ignore_stdin {
        return Ok(CmprssInput::Pipe(std::io::stdin()));
    }
    bail!("No input specified");
}

/// Whether we can send the output to stdout (piped, and the user hasn't
/// suppressed pipe inference).
fn stdout_pipe_usable(args: &CommonArgs) -> bool {
    !std::io::stdout().is_terminal() && !args.ignore_pipes && !args.ignore_stdout
}

/// Defines a single compress/extract action to take.
#[derive(Debug)]
pub struct Job {
    pub compressor: Box<dyn Compressor>,
    pub input: CmprssInput,
    pub output: CmprssOutput,
    pub action: Action,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Action {
    Compress,
    Extract,
    /// Print the archive's file listing to stdout. Only meaningful for
    /// container formats; stream codecs fall through to `Compressor::list`'s
    /// default bail.
    List,
    /// Append new entries to an existing archive. Only supported by container
    /// formats that can grow in place (tar, zip).
    Append,
}

/// Parse the common args and determine the details of the job requested.
///
/// The resolution has three phases:
/// 1. Collect explicit signals from CLI flags (action hint, `-i`/`-o`, the
///    positional `io_list`) into an input list and an optional output path.
/// 2. Build the final `CmprssInput` (falling back to stdin if no paths).
/// 3. Decide the output and the missing pieces of (compressor, action). This
///    branches on how the output is determined: an explicit path, stdout pipe,
///    or a filename we invent from the resolved compressor + action.
pub fn get_job(compressor: Option<Box<dyn Compressor>>, common_args: &CommonArgs) -> Result<Job> {
    // --list short-circuits the output/action machinery: there's no output
    // file, the action is fixed, and we only need the input and compressor.
    if common_args.list {
        let (input_paths, _) = partition_paths(common_args, Some(Action::List))?;
        let input = resolve_input(input_paths, common_args)?;
        let compressor = compressor
            .or_else(|| get_compressor_from_filename(get_input_filename(&input).ok()?))
            .ok_or_else(|| anyhow!("Could not determine compressor to use"))?;
        return Ok(Job {
            compressor,
            input,
            // List writes to stdout directly; this output slot is unused.
            output: CmprssOutput::Pipe(std::io::stdout()),
            action: Action::List,
        });
    }

    let action_hint = action_from_flags(common_args);
    let (input_paths, output_path) = partition_paths(common_args, action_hint)?;
    let input = resolve_input(input_paths, common_args)?;

    // Branch 1: user gave us an output path. Resolve compressor + action
    // using both sides' extensions.
    if let Some(path) = output_path {
        let output = CmprssOutput::Path(path);
        let (compressor, action) = finalize_with_output(compressor, action_hint, &input, &output)?;
        return Ok(Job {
            compressor,
            input,
            output,
            action,
        });
    }

    // Branch 2: stdout is a pipe. Same resolution, but the output has no path.
    if stdout_pipe_usable(common_args) {
        let output = CmprssOutput::Pipe(std::io::stdout());
        let (compressor, action) = finalize_with_output(compressor, action_hint, &input, &output)?;
        return Ok(Job {
            compressor,
            input,
            output,
            action,
        });
    }

    // Branch 3: no output and stdout is a terminal. We must invent a filename,
    // which requires the compressor and action up front.
    let (compressor, action) = finalize_without_output(compressor, action_hint, &input)?;
    let default_name = match action {
        Action::Compress => compressor.default_compressed_filename(get_input_filename(&input)?),
        Action::Extract => compressor.default_extracted_filename(get_input_filename(&input)?),
        // List short-circuits above; finalize_without_output never returns it.
        Action::List => unreachable!("List is handled before Branch 3"),
        // Append without a target archive path has nothing to append to;
        // `finalize_without_output` rejects it before reaching here.
        Action::Append => unreachable!("Append requires an existing output path"),
    };
    Ok(Job {
        compressor,
        input,
        output: CmprssOutput::Path(PathBuf::from(default_name)),
        action,
    })
}

/// Finalize compressor + action when the output is already materialized
/// (either `CmprssOutput::Path` or `CmprssOutput::Pipe`).
fn finalize_with_output(
    mut compressor: Option<Box<dyn Compressor>>,
    mut action: Option<Action>,
    input: &CmprssInput,
    output: &CmprssOutput,
) -> Result<(Box<dyn Compressor>, Action)> {
    if compressor.is_none() || action.is_none() {
        fill_missing_from_io(&mut compressor, &mut action, input, output)?;
    }
    let compressor = compressor.ok_or_else(|| anyhow!("Could not determine compressor to use"))?;
    let action = action.ok_or_else(|| anyhow!("Could not determine action to take"))?;
    Ok((compressor, action))
}

/// Finalize compressor + action when no output path is known (stdout is a
/// terminal and we'll invent a filename next). All inference must come from
/// the input side.
fn finalize_without_output(
    compressor: Option<Box<dyn Compressor>>,
    action: Option<Action>,
    input: &CmprssInput,
) -> Result<(Box<dyn Compressor>, Action)> {
    let input_path = get_input_filename(input)?;
    match action {
        Some(Action::Compress) => {
            let c = compressor.ok_or_else(|| anyhow!("Could not determine compressor to use"))?;
            Ok((c, Action::Compress))
        }
        Some(Action::Extract) => {
            let c = compressor
                .or_else(|| get_compressor_from_filename(input_path))
                .ok_or_else(|| anyhow!("Could not determine compressor to use"))?;
            Ok((c, Action::Extract))
        }
        // Append needs an existing archive path to grow; without one there's
        // nothing to append to.
        Some(Action::Append) => {
            bail!("--append requires an existing archive as the output target")
        }
        // List is handled by the short-circuit at the top of get_job and
        // never flows into this helper.
        Some(Action::List) => unreachable!("List is handled before Branch 3"),
        None => match compressor {
            Some(c) => {
                // Compare the compressor's extension against the input's.
                let action = match get_compressor_from_filename(input_path) {
                    Some(ic) if ic.name() == c.name() => Action::Extract,
                    _ => Action::Compress,
                };
                Ok((c, action))
            }
            None => {
                // The input has to be something we can identify as an archive.
                let c = get_compressor_from_filename(input_path)
                    .ok_or_else(|| anyhow!("Could not determine compressor to use"))?;
                Ok((c, Action::Extract))
            }
        },
    }
}

/// Fill in a missing compressor and/or action by inspecting the input and
/// output shapes. Called after the output is known; covers every combination
/// of (Path, Pipe) input × (Path, Pipe) output.
fn fill_missing_from_io(
    compressor: &mut Option<Box<dyn Compressor>>,
    action: &mut Option<Action>,
    input: &CmprssInput,
    output: &CmprssOutput,
) -> Result {
    match *action {
        Some(Action::Compress) => {
            if let CmprssOutput::Path(path) = output {
                *compressor = get_compressor_from_filename(path);
            }
        }
        // List is handled by the short-circuit at the top of get_job.
        Some(Action::List) => unreachable!("List is handled before fill_missing_from_io"),
        Some(Action::Append) => {
            // Append needs an existing archive path to grow; infer the
            // compressor from that path's extension when it wasn't given.
            match output {
                CmprssOutput::Path(path) => {
                    if compressor.is_none() {
                        *compressor = get_compressor_from_filename(path);
                    }
                }
                _ => bail!("--append requires an archive path, not a pipe, as the target"),
            }
        }
        Some(Action::Extract) => {
            if let CmprssInput::Path(paths) = input {
                let [archive_path] = paths.as_slice() else {
                    bail!("Expected exactly one input archive");
                };
                *compressor = get_compressor_from_filename(archive_path);
            }
        }
        None => match (input, output) {
            (CmprssInput::Path(paths), CmprssOutput::Path(path)) => match paths.as_slice() {
                [single] if path.is_dir() => {
                    *compressor = get_compressor_from_filename(single);
                    *action = Some(Action::Extract);
                    if compressor.is_none() {
                        bail!("Could not determine compressor for {:?}", single);
                    }
                }
                _ => {
                    let (c, a) = guess_from_filenames(paths, path, compressor.take())?;
                    *compressor = Some(c);
                    *action = Some(a);
                }
            },
            (CmprssInput::Path(paths), CmprssOutput::Pipe(_)) => {
                // `resolve_input` guarantees `paths` is non-empty when it
                // returns `CmprssInput::Path`, so `first()` is always Some —
                // but surface a clean error instead of relying on the invariant.
                let first = paths
                    .first()
                    .ok_or_else(|| anyhow!("No input file specified"))?;
                if let Some(c) = compressor.as_deref() {
                    *action = Some(match get_compressor_from_filename(first) {
                        Some(ic) if ic.name() == c.name() => Action::Extract,
                        _ => Action::Compress,
                    });
                } else {
                    if paths.len() != 1 {
                        bail!("Expected exactly one input file when writing to stdout");
                    }
                    *compressor = get_compressor_from_filename(first);
                    if compressor.is_some() {
                        *action = Some(Action::Extract);
                    } else {
                        bail!("Could not determine compressor to use");
                    }
                }
            }
            (CmprssInput::Pipe(_), CmprssOutput::Path(path)) => {
                if let Some(c) = compressor.as_deref() {
                    *action = Some(
                        if get_compressor_from_filename(path)
                            .is_some_and(|pc| c.name() == pc.name())
                        {
                            Action::Compress
                        } else {
                            Action::Extract
                        },
                    );
                } else {
                    *compressor = get_compressor_from_filename(path);
                    if compressor.is_some() {
                        *action = Some(Action::Compress);
                    } else {
                        bail!("Could not determine compressor to use");
                    }
                }
            }
            (CmprssInput::Pipe(_), CmprssOutput::Pipe(_)) => {
                *action = Some(Action::Compress);
            }
            // Writer output and Reader input are only constructed internally
            // by the Pipeline compressor; they don't reach get_job from main.
            (_, CmprssOutput::Writer(_)) => *action = Some(Action::Compress),
            (CmprssInput::Reader(_), _) => *action = Some(Action::Extract),
        },
    }
    Ok(())
}

/// Get the input filename or return a default file
/// This file will be used to generate the output filename
fn get_input_filename(input: &CmprssInput) -> Result<&Path> {
    match input {
        CmprssInput::Path(paths) => match paths.first() {
            Some(path) => Ok(path),
            None => bail!("No input specified"),
        },
        CmprssInput::Pipe(_) => Ok(Path::new("archive")),
        CmprssInput::Reader(_) => Ok(Path::new("piped_data")),
    }
}

/// Get a compressor pipeline from a filename by scanning extensions right-to-left
pub fn get_compressor_from_filename(filename: &Path) -> Option<Box<dyn Compressor>> {
    let file_name = filename.file_name()?.to_str()?;
    let parts: Vec<&str> = file_name.split('.').collect();

    if parts.len() < 2 {
        return None;
    }

    // Scan extensions right-to-left, collecting known compressors
    // until hitting an unknown extension or the base name.
    // e.g., "a.b.tar.gz" → gz ✓, tar ✓, b ✗ stop → [gz, tar]
    // `chain_from_ext` handles both single-codec extensions and compound
    // shortcuts like `tgz` (which expand to `[tar, gz]`).
    let mut chain: Vec<Box<dyn Compressor>> = Vec::new();
    for ext in parts[1..].iter().rev() {
        match backends::chain_from_ext(ext) {
            Some(stage) => {
                // stage is innermost→outermost; we walk the filename
                // right-to-left so we push outermost first.
                for c in stage.into_iter().rev() {
                    chain.push(c);
                }
            }
            None => break,
        }
    }

    if chain.is_empty() {
        return None;
    }

    // Reverse to innermost-to-outermost order
    chain.reverse();
    Some(Box::new(Pipeline::new(chain)))
}

/// Convert an input path into a Path
fn get_path(input: &str) -> Option<PathBuf> {
    let path = PathBuf::from(input);
    if !path.try_exists().unwrap_or(false) {
        return None;
    }
    Some(path)
}

/// Guess compressor/action from the two filenames. The compressor may already
/// be given via the subcommand.
///
/// Returns an error when the two filenames don't give enough information to
/// pick an action (e.g. the same format on both sides and the output isn't a
/// directory).
fn guess_from_filenames(
    input: &[PathBuf],
    output: &Path,
    compressor: Option<Box<dyn Compressor>>,
) -> Result<(Box<dyn Compressor>, Action)> {
    let input = match input {
        [single] => single,
        _ => {
            if let Some(c) = get_compressor_from_filename(output) {
                return Ok((c, Action::Compress));
            }
            if output.is_dir()
                && let Some(first) = input.first()
                && let Some(c) = get_compressor_from_filename(first)
            {
                return Ok((c, Action::Extract));
            }
            // No extension hint anywhere, but we were given a compressor —
            // assume the user wants to extract multiple archives to a directory.
            let c = compressor.ok_or_else(|| anyhow!("Could not determine compressor to use"))?;
            return Ok((c, Action::Extract));
        }
    };

    let output_guess = get_compressor_from_filename(output);
    let input_guess = get_compressor_from_filename(input);

    // If the user supplied a compressor via subcommand, pick the action by
    // matching its name against the input/output extensions.
    if let Some(c) = compressor {
        let action = if output_guess
            .as_ref()
            .is_some_and(|og| og.name() == c.name())
        {
            Action::Compress
        } else if input_guess.as_ref().is_some_and(|ig| ig.name() == c.name()) {
            Action::Extract
        } else {
            // Extensions don't match on either side; default to compressing.
            Action::Compress
        };
        return Ok((c, action));
    }

    match (output_guess, input_guess) {
        (None, None) => bail!("Could not determine compressor to use"),
        (Some(c), None) => Ok((c, Action::Compress)),
        (None, Some(e)) => Ok((e, Action::Extract)),
        (Some(c), Some(e)) => {
            // Both sides carry a known extension — decide whether this is
            // adding or stripping a single outer layer (e.g. tar → tar.gz).
            let input_file = input
                .file_name()
                .and_then(|f| f.to_str())
                .ok_or_else(|| anyhow!("Could not parse input filename"))?;
            let input_ext = input
                .extension()
                .and_then(|e| e.to_str())
                .ok_or_else(|| anyhow!("Could not parse input extension"))?;
            let output_file = output
                .file_name()
                .and_then(|f| f.to_str())
                .ok_or_else(|| anyhow!("Could not parse output filename"))?;
            let output_ext = output
                .extension()
                .and_then(|e| e.to_str())
                .ok_or_else(|| anyhow!("Could not parse output extension"))?;
            let layer_added = format!("{input_file}.{output_ext}");
            let layer_stripped = format!("{output_file}.{input_ext}");

            if layer_added == output_file {
                // input="archive.tar", output="archive.tar.gz" — add the outer layer only.
                let single = backends::compressor_from_str(output_ext).unwrap_or(c);
                Ok((single, Action::Compress))
            } else if layer_stripped == input_file {
                // input="archive.tar.gz", output="archive.tar" — strip the outer layer only.
                let single = backends::compressor_from_str(input_ext).unwrap_or(e);
                Ok((single, Action::Extract))
            } else if c.name() == e.name() {
                // Same format on both sides: only meaningful when the output
                // is a directory (extracting in place).
                if output.is_dir() {
                    Ok((e, Action::Extract))
                } else {
                    bail!("Could not determine action to take");
                }
            } else if output.is_dir() {
                Ok((e, Action::Extract))
            } else {
                bail!("Could not determine action to take");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::ExtractedTarget;
    use std::path::Path;

    fn compressor_name(path: &str) -> Option<String> {
        get_compressor_from_filename(Path::new(path)).map(|c| c.name().to_string())
    }

    fn compressor_extension(path: &str) -> Option<String> {
        get_compressor_from_filename(Path::new(path)).map(|c| c.extension().to_string())
    }

    #[test]
    fn test_single_extension() {
        assert_eq!(compressor_name("file.gz"), Some("gzip".into()));
        assert_eq!(compressor_name("file.xz"), Some("xz".into()));
        assert_eq!(compressor_name("file.bz2"), Some("bzip2".into()));
        assert_eq!(compressor_name("file.zst"), Some("zstd".into()));
        assert_eq!(compressor_name("file.lz4"), Some("lz4".into()));
        assert_eq!(compressor_name("file.br"), Some("brotli".into()));
        assert_eq!(compressor_name("file.sz"), Some("snappy".into()));
        assert_eq!(compressor_name("file.lzma"), Some("lzma".into()));
        assert_eq!(compressor_name("file.tar"), Some("tar".into()));
        assert_eq!(compressor_name("file.zip"), Some("zip".into()));
    }

    #[test]
    fn test_multi_extension() {
        assert_eq!(compressor_name("archive.tar.gz"), Some("gzip".into()));
        assert_eq!(compressor_name("archive.tar.xz"), Some("xz".into()));
        assert_eq!(compressor_name("archive.tar.bz2"), Some("bzip2".into()));
        assert_eq!(compressor_name("archive.tar.zst"), Some("zstd".into()));
    }

    #[test]
    fn test_shortcut_extensions() {
        // Shortcut extensions resolve to a tar + outer compressor pipeline,
        // so the reported name is the outer compressor (same as the long form).
        assert_eq!(compressor_name("archive.tgz"), Some("gzip".into()));
        assert_eq!(compressor_name("archive.tbz"), Some("bzip2".into()));
        assert_eq!(compressor_name("archive.tbz2"), Some("bzip2".into()));
        assert_eq!(compressor_name("archive.txz"), Some("xz".into()));
        assert_eq!(compressor_name("archive.tzst"), Some("zstd".into()));
    }

    #[test]
    fn test_shortcut_extensions_extract_to_directory() {
        // Shortcuts are tar-based, so they must extract to a directory.
        for path in ["a.tgz", "a.tbz", "a.tbz2", "a.txz", "a.tzst"] {
            let c = get_compressor_from_filename(Path::new(path)).unwrap();
            assert_eq!(
                c.default_extracted_target(),
                ExtractedTarget::Directory,
                "{path} should extract to a directory",
            );
        }
    }

    #[test]
    fn test_unknown_middle_extension() {
        // "b" is not a compressor, so only tar.gz should be detected
        assert_eq!(compressor_name("a.b.tar.gz"), Some("gzip".into()));
        assert_eq!(compressor_name("report.2024.tar.gz"), Some("gzip".into()));
    }

    #[test]
    fn test_no_recognized_extension() {
        assert_eq!(compressor_name("file.txt"), None);
        assert_eq!(compressor_name("file.pdf"), None);
        assert_eq!(compressor_name("file"), None);
    }

    #[test]
    fn test_default_filenames_single_pipeline() {
        let c = get_compressor_from_filename(Path::new("file.gz")).unwrap();
        assert_eq!(
            c.default_compressed_filename(Path::new("data.txt")),
            "data.txt.gz"
        );
        assert_eq!(c.default_extracted_filename(Path::new("data.gz")), "data");
    }

    #[test]
    fn test_default_filenames_multi_pipeline() {
        let c = get_compressor_from_filename(Path::new("archive.tar.gz")).unwrap();
        assert_eq!(
            c.default_compressed_filename(Path::new("data")),
            "data.tar.gz"
        );
        // tar.gz extracts to a directory, so extracted filename is "."
        assert_eq!(c.default_extracted_filename(Path::new("data.tar.gz")), ".");
    }

    #[test]
    fn test_is_archive_single_pipeline() {
        let c = get_compressor_from_filename(Path::new("file.gz")).unwrap();
        assert!(c.is_archive(Path::new("test.gz")));
        assert!(!c.is_archive(Path::new("test.xz")));
    }

    #[test]
    fn test_is_archive_multi_pipeline() {
        let c = get_compressor_from_filename(Path::new("archive.tar.gz")).unwrap();
        assert!(c.is_archive(Path::new("foo.tar.gz")));
        assert!(!c.is_archive(Path::new("foo.gz")));
    }

    #[test]
    fn test_extracted_target_single_pipeline() {
        let gz = get_compressor_from_filename(Path::new("file.gz")).unwrap();
        assert_eq!(gz.default_extracted_target(), ExtractedTarget::File);

        let tar = get_compressor_from_filename(Path::new("file.tar")).unwrap();
        assert_eq!(tar.default_extracted_target(), ExtractedTarget::Directory);
    }

    #[test]
    fn test_extracted_target_multi_pipeline() {
        // tar.gz: innermost is tar, which extracts to directory
        let c = get_compressor_from_filename(Path::new("archive.tar.gz")).unwrap();
        assert_eq!(c.default_extracted_target(), ExtractedTarget::Directory);
    }

    #[test]
    fn test_single_extension_returns_correct_extension() {
        assert_eq!(compressor_extension("file.gz"), Some("gz".into()));
        assert_eq!(compressor_extension("file.tar"), Some("tar".into()));
    }
}
