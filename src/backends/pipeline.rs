use crate::progress::{OutputTarget, ProgressArgs, copy_with_progress};
use crate::utils::{
    CmprssInput, CmprssOutput, Compressor, ExtractedTarget, PassthroughWriter, ReadWrapper, Result,
    StreamWriter, WriteWrapper,
};
use anyhow::{anyhow, bail};
use std::io::{self, Read, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};

/// A pipeline of one or more compressors applied in sequence (e.g., tar.gz).
///
/// The chain is laid out **innermost → outermost**: for `tar.gz`, that's
/// `[tar, gz]` — tar produces archive bytes, gz wraps those bytes. At most
/// one container (tar / zip / sevenz — anything whose
/// [`Compressor::as_stream_codec`] returns `None`) is allowed, and it must
/// sit at the innermost position. Everything outside the container is a
/// [`StreamCodec`](crate::utils::StreamCodec) decorator.
///
/// Compression composes the codecs onto the output writer (innermost wraps
/// first, then each outer codec wraps the previous layer); decompression
/// composes onto the input reader the same way. The container, when
/// present, runs at the boundary on the main thread — there is no thread or
/// channel involved in this pipeline.
pub struct Pipeline {
    compressors: Vec<Box<dyn Compressor>>,
    /// Preserves the user's original format string (e.g. `tgz`) so default
    /// filenames use it verbatim instead of the dotted composition of each
    /// stage's extension. `None` falls back to joining the per-stage
    /// extensions.
    format_override: Option<String>,
    /// Explicit progress override applied to the codec-only data-copy paths
    /// (compress + extract). When set, takes precedence over the outermost
    /// codec's per-stage `progress_args()`. CLI codec-only invocations
    /// (`cmprss gz.xz file`) populate this from the shared `--progress`
    /// flag; library callers can leave it unset and configure progress per
    /// stage instead.
    progress_args: Option<ProgressArgs>,
}

/// `(innermost_container, surrounding_codecs)` split of a pipeline chain.
type SplitChain<'a> = (Option<&'a dyn Compressor>, &'a [Box<dyn Compressor>]);

impl Clone for Pipeline {
    fn clone(&self) -> Self {
        Pipeline {
            compressors: self.compressors.iter().map(|c| c.clone_boxed()).collect(),
            format_override: self.format_override.clone(),
            progress_args: self.progress_args,
        }
    }
}

impl Pipeline {
    pub fn new(compressors: Vec<Box<dyn Compressor>>) -> Self {
        Pipeline {
            compressors,
            format_override: None,
            progress_args: None,
        }
    }

    /// Create a Pipeline that keeps `format` as its canonical format string,
    /// used for default output filenames. Intended for shortcut forms like
    /// `tgz` where the user-facing extension differs from the dotted chain.
    pub fn with_format(compressors: Vec<Box<dyn Compressor>>, format: String) -> Self {
        Pipeline {
            compressors,
            format_override: Some(format),
            progress_args: None,
        }
    }

    /// Attach a progress configuration to this pipeline. Used by the CLI to
    /// thread the shared `--progress` / `--chunk-size` flags through to the
    /// codec-only copy path; if unset, the pipeline falls back to the
    /// outermost codec's own `progress_args()`.
    pub fn with_progress_args(mut self, progress_args: ProgressArgs) -> Self {
        self.progress_args = Some(progress_args);
        self
    }

    /// Resolve the `ProgressArgs` to use for codec-only chains: explicit
    /// override first, then outermost codec's setting, then the default.
    fn resolve_progress(&self, codecs: &[Box<dyn Compressor>]) -> ProgressArgs {
        self.progress_args.unwrap_or_else(|| {
            codecs
                .last()
                .and_then(|c| c.progress_args())
                .copied()
                .unwrap_or_default()
        })
    }

    /// Get a string representation of the chained format (e.g., "tar.gz").
    fn format_chain(&self) -> String {
        if let Some(ref f) = self.format_override {
            return f.clone();
        }
        self.compressors
            .iter()
            .map(|c| c.extension())
            .collect::<Vec<&str>>()
            .join(".")
    }

    /// Split the chain into an optional innermost container and the surrounding
    /// stream codecs. Bails if any non-innermost stage is a container, since
    /// "tar inside gzip outside tar" makes no sense as a composition.
    fn split_chain(&self) -> Result<SplitChain<'_>> {
        debug_assert!(!self.compressors.is_empty(), "pipeline is never empty");
        let first = self.compressors[0].as_ref();
        let (container, codecs) = if first.as_stream_codec().is_none() {
            (Some(first), &self.compressors[1..])
        } else {
            (None, &self.compressors[..])
        };
        for stage in codecs {
            if stage.as_stream_codec().is_none() {
                bail!(
                    "pipeline contains a non-stream stage ({}) outside the innermost position; \
                     container formats (tar, zip, 7z) can only appear as the innermost layer",
                    stage.name()
                );
            }
        }
        Ok((container, codecs))
    }

    /// Wrap a final sink writer with each codec in `codecs`. The slice is
    /// laid out innermost → outermost in archive-layer terms, so the
    /// outermost codec must wrap the sink first (it sits closest to the
    /// file on the write path) and the innermost codec wraps last (it's
    /// closest to the payload).
    ///
    /// Returned `StreamWriter` is the innermost layer; writing through it
    /// cascades outward to `sink`, and calling `finish` finalizes every
    /// layer in sequence.
    fn build_encoder_chain(
        codecs: &[Box<dyn Compressor>],
        sink: Box<dyn Write + Send>,
    ) -> Result<Box<dyn StreamWriter>> {
        let mut chain: Box<dyn StreamWriter> = Box::new(PassthroughWriter(sink));
        for stage in codecs.iter().rev() {
            let codec = stage
                .as_stream_codec()
                .expect("split_chain guarantees stream codecs in this slice");
            chain = codec.encoder(chain)?;
        }
        Ok(chain)
    }

    /// Wrap a source reader with each codec's decoder in outermost → innermost
    /// order — i.e. the reverse of compression order, since decoding peels
    /// layers from the outside in. The resulting `Read` yields the
    /// fully-decoded byte stream.
    fn build_decoder_chain(
        codecs: &[Box<dyn Compressor>],
        source: Box<dyn Read + Send>,
    ) -> Result<Box<dyn Read + Send>> {
        let mut chain = source;
        for stage in codecs.iter().rev() {
            let codec = stage
                .as_stream_codec()
                .expect("split_chain guarantees stream codecs in this slice");
            chain = codec.decoder(chain)?;
        }
        Ok(chain)
    }
}

/// A `Write` that owns a `StreamWriter` chain and finalizes it on Drop,
/// stashing any finish error in a shared slot. The pipeline reads the slot
/// after the container's `compress` returns to surface finalize errors.
///
/// This pattern exists because the `Compressor::compress` API hands a
/// `Box<dyn Write + Send>` to the container; the container owns the box for
/// the duration of its work and drops it when it returns. Drop is the only
/// hook we have to drive the cascade-finalize without changing that API.
struct FinalizeOnDrop {
    inner: Option<Box<dyn StreamWriter>>,
    slot: Arc<Mutex<Option<io::Result<()>>>>,
}

impl Write for FinalizeOnDrop {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner
            .as_mut()
            .expect("inner taken before drop")
            .write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.inner
            .as_mut()
            .expect("inner taken before drop")
            .flush()
    }
}

impl Drop for FinalizeOnDrop {
    fn drop(&mut self) {
        if let Some(inner) = self.inner.take() {
            *self.slot.lock().unwrap() = Some(inner.finish());
        }
    }
}

/// Open the original sink writer for the user-supplied `CmprssOutput`, along
/// with the `OutputTarget` describing how a progress bar should treat it
/// (file → show, stdout → suppress in `Auto` mode, in-memory → no bar).
fn open_sink(output: CmprssOutput) -> Result<(Box<dyn Write + Send>, OutputTarget)> {
    use std::fs::File;
    use std::io::BufWriter;
    match output {
        CmprssOutput::Writer(WriteWrapper(w)) => Ok((w, OutputTarget::InMemory)),
        CmprssOutput::Pipe(stdout) => Ok((Box::new(BufWriter::new(stdout)), OutputTarget::Stdout)),
        CmprssOutput::Path(path) => Ok((
            Box::new(BufWriter::new(File::create(path)?)),
            OutputTarget::File,
        )),
    }
}

/// Open the original source reader for the user-supplied `CmprssInput` along
/// with the input file's size when known (used by the codec-only progress
/// bar; `None` for stdin and in-memory readers). For path inputs we require
/// exactly one file (single-stream codecs don't accept multiple inputs);
/// container-led extracts route through `input` unchanged before this is
/// called, so directory inputs never reach here.
fn open_source(input: CmprssInput, name: &str) -> Result<(Box<dyn Read + Send>, Option<u64>)> {
    use std::fs::File;
    use std::io::BufReader;
    match input {
        CmprssInput::Path(paths) => {
            if paths.len() != 1 {
                bail!("{name} expects a single input file");
            }
            if paths[0].is_dir() {
                bail!("{name} does not operate on directories");
            }
            let size = std::fs::metadata(&paths[0])?.len();
            Ok((Box::new(BufReader::new(File::open(&paths[0])?)), Some(size)))
        }
        CmprssInput::Pipe(stdin) => Ok((Box::new(BufReader::new(stdin)), None)),
        CmprssInput::Reader(ReadWrapper(r)) => Ok((r, None)),
    }
}

impl Compressor for Pipeline {
    fn name(&self) -> &str {
        self.compressors
            .last()
            .expect("pipeline is never empty")
            .name()
    }

    fn extension(&self) -> &str {
        self.compressors
            .last()
            .expect("pipeline is never empty")
            .extension()
    }

    fn default_extracted_target(&self) -> ExtractedTarget {
        self.compressors
            .first()
            .expect("pipeline is never empty")
            .default_extracted_target()
    }

    fn default_compressed_filename(&self, in_path: &Path) -> String {
        // Add all extensions: input.txt → input.txt.tar.gz
        let base = in_path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "archive".to_string());
        format!("{}.{}", base, self.format_chain())
    }

    fn default_extracted_filename(&self, in_path: &Path) -> String {
        if self.default_extracted_target() == ExtractedTarget::Directory {
            return ".".to_string();
        }
        // Strip all known extensions: input.tar.gz → input
        let mut name = in_path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "archive".to_string());
        for comp in self.compressors.iter().rev() {
            let ext = format!(".{}", comp.extension());
            if let Some(stripped) = name.strip_suffix(&ext) {
                name = stripped.to_string();
            }
        }
        name
    }

    fn is_archive(&self, in_path: &Path) -> bool {
        let file_name = match in_path.file_name().and_then(|f| f.to_str()) {
            Some(f) => f,
            None => return false,
        };
        file_name.ends_with(&format!(".{}", self.format_chain()))
    }

    fn compress(&self, input: CmprssInput, output: CmprssOutput) -> Result {
        debug_assert!(!self.compressors.is_empty(), "pipeline is never empty");
        if self.compressors.len() == 1 {
            return self.compressors[0].compress(input, output);
        }
        let (container, codecs) = self.split_chain()?;

        match container {
            Some(c) => {
                // Hand the container a writer that finalizes the codec chain
                // when it goes out of scope (i.e. when the container's
                // compress returns), and surface any finalize error.
                let (sink, _target) = open_sink(output)?;
                let chain = Self::build_encoder_chain(codecs, sink)?;
                let slot = Arc::new(Mutex::new(None));
                let handle = FinalizeOnDrop {
                    inner: Some(chain),
                    slot: slot.clone(),
                };
                c.compress(input, CmprssOutput::Writer(WriteWrapper(Box::new(handle))))?;
                if let Some(result) = slot.lock().unwrap().take() {
                    result?;
                } else {
                    bail!("pipeline finalize never fired: container retained the writer");
                }
                Ok(())
            }
            None => {
                // All-codec chain (e.g. `.gz.xz`): drive a progress bar over
                // the raw source-byte reads, mirroring the single-codec
                // `stream_compress` path. We borrow the outermost codec's
                // `ProgressArgs` because it sits closest to disk on the
                // write path and is what a user would set for the equivalent
                // single-codec invocation. Library callers configuring a
                // Pipeline directly can pick whatever stage they want by
                // setting that stage's `progress_args`.
                let (source, input_size) = open_source(input, self.name())?;
                let (sink, target) = open_sink(output)?;
                let chain = Self::build_encoder_chain(codecs, sink)?;
                let progress = self.resolve_progress(codecs);
                let mut chain = chain;
                copy_with_progress(
                    source,
                    &mut chain,
                    progress.chunk_size.size_in_bytes,
                    input_size,
                    progress.progress,
                    target,
                )?;
                chain.finish()?;
                Ok(())
            }
        }
    }

    fn extract(&self, input: CmprssInput, output: CmprssOutput) -> Result {
        debug_assert!(!self.compressors.is_empty(), "pipeline is never empty");
        if self.compressors.len() == 1 {
            return self.compressors[0].extract(input, output);
        }
        let (container, codecs) = self.split_chain()?;
        let (source, input_size) = open_source(input, self.name())?;
        let chain = Self::build_decoder_chain(codecs, source)?;

        match container {
            Some(c) => {
                // The innermost container reads decoded bytes from the chain
                // and unpacks to the user-supplied output. If the output is a
                // directory path that doesn't exist, create it so that e.g.
                // `tar::unpack` has somewhere to write.
                let final_output = match output {
                    CmprssOutput::Path(ref p) => {
                        if c.default_extracted_target() == ExtractedTarget::Directory && !p.exists()
                        {
                            std::fs::create_dir_all(p)?;
                        }
                        CmprssOutput::Path(p.clone())
                    }
                    CmprssOutput::Pipe(_) | CmprssOutput::Writer(_) => output,
                };
                c.extract(CmprssInput::Reader(ReadWrapper(chain)), final_output)
            }
            None => {
                // All-codec chain: drive a progress bar against the encoded
                // archive size (the only size we know up front; the decoded
                // total isn't available until we've decoded it). Same
                // outermost-codec source for progress settings as compress().
                let (sink, target) = open_sink(output)?;
                let progress = self.resolve_progress(codecs);
                copy_with_progress(
                    chain,
                    sink,
                    progress.chunk_size.size_in_bytes,
                    input_size,
                    progress.progress,
                    target,
                )?;
                Ok(())
            }
        }
    }

    fn append(&self, input: CmprssInput, output: CmprssOutput) -> Result {
        debug_assert!(!self.compressors.is_empty(), "pipeline is never empty");
        if self.compressors.len() == 1 {
            // Single-stage pipelines are just a wrapper; delegate so tar/zip
            // reached via positional-path inference still support --append.
            return self.compressors[0].append(input, output);
        }
        bail!(
            "cannot --append to a compound archive ({}); it would require decompressing and recompressing the whole archive",
            self.format_chain()
        )
    }

    fn list(&self, input: CmprssInput) -> Result {
        debug_assert!(!self.compressors.is_empty(), "pipeline is never empty");
        if self.compressors.len() == 1 {
            return self.compressors[0].list(input);
        }
        let (container, codecs) = self.split_chain()?;
        let (source, _input_size) = open_source(input, self.name())?;
        let chain = Self::build_decoder_chain(codecs, source)?;
        match container {
            Some(c) => c.list(CmprssInput::Reader(ReadWrapper(chain))),
            None => Err(anyhow!(
                "{} archives cannot be listed; only container formats (tar, zip) support --list",
                self.format_chain()
            )),
        }
    }
}

#[cfg(test)]
#[allow(unused_imports)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[cfg(all(feature = "tar", feature = "gzip"))]
    #[test]
    fn test_pipeline_compression() -> Result {
        let temp_dir = tempdir()?;

        let test_content = "This is a test file for pipeline compression";
        let test_file_path = temp_dir.path().join("test.txt");
        fs::write(&test_file_path, test_content)?;

        let pipeline = Pipeline::new(vec![
            Box::new(crate::backends::Tar::default()),
            Box::new(crate::backends::Gzip::default()),
        ]);

        let archive_path = temp_dir.path().join("test.tar.gz");
        pipeline.compress(
            CmprssInput::Path(vec![test_file_path.clone()]),
            CmprssOutput::Path(archive_path.clone()),
        )?;

        assert!(archive_path.exists());

        let output_dir = temp_dir.path().join("extracted");
        fs::create_dir(&output_dir)?;
        pipeline.extract(
            CmprssInput::Path(vec![archive_path.clone()]),
            CmprssOutput::Path(output_dir.clone()),
        )?;

        let extracted_file = output_dir.join("test.txt");
        assert!(extracted_file.exists());

        let extracted_content = fs::read_to_string(extracted_file)?;
        assert_eq!(extracted_content, test_content);

        Ok(())
    }

    /// Regression test: per-stage configuration (e.g. `--level 1` vs
    /// `--level 9` on the outer gzip of a `.tar.gz`) must survive the
    /// composition in `Pipeline::compress`. Earlier the pipeline reconstructed
    /// each stage from its *name* alone, producing a default Gzip regardless
    /// of the level the user requested; the StreamCodec rewrite uses the
    /// per-stage config directly, so this test guards against regressions in
    /// either direction.
    #[cfg(all(feature = "tar", feature = "gzip"))]
    #[test]
    fn test_pipeline_preserves_stage_config() -> Result {
        use crate::progress::ProgressArgs;

        let temp_dir = tempdir()?;
        let input = temp_dir.path().join("input.txt");
        // Repetitive content amplifies the level difference in output size.
        fs::write(&input, "0123456789abcdef".repeat(1024))?;

        let run = |level: i32, suffix: &str| -> Result<u64> {
            let fast = Pipeline::new(vec![
                Box::new(crate::backends::Tar::default()),
                Box::new(crate::backends::Gzip {
                    compression_level: level,
                    progress_args: ProgressArgs::default(),
                }),
            ]);
            let out = temp_dir.path().join(format!("out.{suffix}.tar.gz"));
            fast.compress(
                CmprssInput::Path(vec![input.clone()]),
                CmprssOutput::Path(out.clone()),
            )?;
            Ok(fs::metadata(&out)?.len())
        };

        let fast_size = run(1, "fast")?;
        let best_size = run(9, "best")?;
        assert!(
            best_size < fast_size,
            "expected best (level 9) to be smaller than fast (level 1), got {best_size} >= {fast_size}",
        );

        Ok(())
    }

    /// A multi-codec chain (no container) should still round-trip cleanly:
    /// raw bytes → gz → xz → file, then file → xz → gz → raw bytes.
    #[cfg(all(feature = "gzip", feature = "xz"))]
    #[test]
    fn test_pipeline_codec_only_roundtrip() -> Result {
        let temp_dir = tempdir()?;
        let input = temp_dir.path().join("input.bin");
        let payload: Vec<u8> = (0u8..=255).cycle().take(64 * 1024).collect();
        fs::write(&input, &payload)?;

        let pipeline = Pipeline::new(vec![
            Box::new(crate::backends::Gzip::default()),
            Box::new(crate::backends::Xz::default()),
        ]);

        let archive = temp_dir.path().join("input.bin.gz.xz");
        pipeline.compress(
            CmprssInput::Path(vec![input.clone()]),
            CmprssOutput::Path(archive.clone()),
        )?;
        assert!(archive.exists());

        let extracted = temp_dir.path().join("input.bin.recovered");
        pipeline.extract(
            CmprssInput::Path(vec![archive.clone()]),
            CmprssOutput::Path(extracted.clone()),
        )?;
        let recovered = fs::read(&extracted)?;
        assert_eq!(recovered, payload);

        Ok(())
    }
}
