use crate::utils::{
    CmprssInput, CmprssOutput, Compressor, ExtractedTarget, ReadWrapper, Result, WriteWrapper,
};
use anyhow::anyhow;
use std::io::{self, Read, Write};
use std::path::Path;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread;

/// A pipeline of one or more compressors applied in sequence (e.g., tar.gz)
pub struct Pipeline {
    // The chain of compressors to apply in order (innermost to outermost)
    compressors: Vec<Box<dyn Compressor>>,
}

impl Clone for Pipeline {
    fn clone(&self) -> Self {
        Pipeline {
            compressors: self.compressors.iter().map(|c| c.clone_boxed()).collect(),
        }
    }
}

/// Which method intermediate (threaded) stages should invoke. The final stage
/// always runs on the calling thread and is handled by a caller-supplied
/// closure — only the intermediate layers need this dispatch.
#[derive(Clone, Copy)]
enum StageAction {
    Compress,
    Extract,
}

impl Pipeline {
    /// Create a new Pipeline with the given compressors
    pub fn new(compressors: Vec<Box<dyn Compressor>>) -> Self {
        Pipeline { compressors }
    }

    /// Get a string representation of the chained format (e.g., "tar.gz")
    fn format_chain(&self) -> String {
        self.compressors
            .iter()
            .map(|c| c.extension())
            .collect::<Vec<&str>>()
            .join(".")
    }

    /// Run an ordered chain of compressor stages, with each non-final stage
    /// in its own thread linked by an in-memory pipe. The final (last) stage
    /// runs on the calling thread via `finalize`. Intermediate stages all
    /// invoke the same method — `compress` going outward through a
    /// compression pipeline, `extract` unwrapping layers on the way in.
    fn run_threaded<F>(
        stages: Vec<Box<dyn Compressor>>,
        initial_input: CmprssInput,
        intermediate: StageAction,
        finalize: F,
    ) -> Result
    where
        F: FnOnce(Box<dyn Compressor>, CmprssInput) -> Result,
    {
        debug_assert!(!stages.is_empty(), "pipeline is never empty");
        let mut stages = stages;
        let last = stages.pop().expect("pipeline is never empty");
        let buffer_size = 64 * 1024;
        let mut current_input = initial_input;
        let mut handles = Vec::new();

        for stage in stages {
            let (sender, receiver) = channel::<Vec<u8>>();
            let stage_output =
                CmprssOutput::Writer(WriteWrapper(Box::new(PipeWriter::new(sender, buffer_size))));
            let next_input = CmprssInput::Reader(ReadWrapper(Box::new(PipeReader::new(receiver))));
            let stage_input = std::mem::replace(&mut current_input, next_input);

            let handle = thread::spawn(move || match intermediate {
                StageAction::Compress => stage.compress(stage_input, stage_output),
                StageAction::Extract => stage.extract(stage_input, stage_output),
            });
            handles.push(handle);
        }

        finalize(last, current_input)?;

        for handle in handles {
            handle
                .join()
                .map_err(|_| anyhow!("Pipeline stage thread panicked"))??;
        }
        Ok(())
    }
}

/// A reader that reads from a receiver channel
struct PipeReader {
    receiver: Receiver<Vec<u8>>,
    buffer: Vec<u8>,
    position: usize,
    eof: bool,
}

impl PipeReader {
    fn new(receiver: Receiver<Vec<u8>>) -> Self {
        PipeReader {
            receiver,
            buffer: Vec::new(),
            position: 0,
            eof: false,
        }
    }
}

impl Read for PipeReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        // If we've reached EOF, return 0 to signal that
        if self.eof && self.position >= self.buffer.len() {
            return Ok(0);
        }

        // If we've consumed the current buffer, try to get a new one
        if self.position >= self.buffer.len() {
            match self.receiver.recv() {
                Ok(data) => {
                    // Empty data signals EOF from the writer
                    if data.is_empty() {
                        self.eof = true;
                        return Ok(0);
                    }
                    self.buffer = data;
                    self.position = 0;
                }
                Err(_) => {
                    // Channel closed, this means EOF
                    self.eof = true;
                    return Ok(0);
                }
            }
        }

        // Copy data from our buffer to the output buffer
        let available = self.buffer.len() - self.position;
        let to_copy = available.min(buf.len());
        buf[..to_copy].copy_from_slice(&self.buffer[self.position..self.position + to_copy]);
        self.position += to_copy;
        Ok(to_copy)
    }
}

/// A writer that writes to a sender channel
struct PipeWriter {
    sender: Sender<Vec<u8>>,
    buffer_size: usize,
}

impl PipeWriter {
    fn new(sender: Sender<Vec<u8>>, buffer_size: usize) -> Self {
        PipeWriter {
            sender,
            buffer_size,
        }
    }
}

impl Write for PipeWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        // Split the input into chunks of buffer_size
        let mut start = 0;
        while start < buf.len() {
            let end = (start + self.buffer_size).min(buf.len());
            let chunk = Vec::from(&buf[start..end]);

            // Send the chunk through the channel
            if self.sender.send(chunk).is_err() {
                // If the receiver is gone, report an error
                return Err(io::Error::new(
                    io::ErrorKind::BrokenPipe,
                    "Pipe receiver has been closed",
                ));
            }
            start = end;
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        // No need to flush, the channel sends immediately
        Ok(())
    }
}

impl Drop for PipeWriter {
    fn drop(&mut self) {
        // Send an empty buffer to signal EOF
        let _ = self.sender.send(Vec::new());
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
            .unwrap_or_else(|| std::ffi::OsStr::new("archive"))
            .to_str()
            .unwrap();
        format!("{}.{}", base, self.format_chain())
    }

    fn default_extracted_filename(&self, in_path: &Path) -> String {
        if self.default_extracted_target() == ExtractedTarget::Directory {
            return ".".to_string();
        }
        // Strip all known extensions: input.tar.gz → input
        let mut name = in_path
            .file_name()
            .unwrap_or_else(|| std::ffi::OsStr::new("archive"))
            .to_str()
            .unwrap()
            .to_string();
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
        // Innermost → outermost: the outermost compressor runs on the main
        // thread and writes to the user-supplied output.
        let stages = self.compressors.iter().map(|c| c.clone_boxed()).collect();
        Self::run_threaded(stages, input, StageAction::Compress, |last, input| {
            last.compress(input, output)
        })
    }

    fn extract(&self, input: CmprssInput, output: CmprssOutput) -> Result {
        debug_assert!(!self.compressors.is_empty(), "pipeline is never empty");
        if self.compressors.len() == 1 {
            return self.compressors[0].extract(input, output);
        }
        // Outermost → innermost: the innermost extractor (typically the
        // container format like tar/zip) runs on the main thread so it can
        // unpack into the user-supplied output.
        let stages = self
            .compressors
            .iter()
            .rev()
            .map(|c| c.clone_boxed())
            .collect();
        Self::run_threaded(stages, input, StageAction::Extract, |last, input| {
            let final_output = match output {
                CmprssOutput::Path(ref p) => {
                    // If the innermost extractor wants a directory and the
                    // user's output path doesn't exist yet, create it so
                    // e.g. tar::unpack has somewhere to write.
                    if last.default_extracted_target() == ExtractedTarget::Directory && !p.exists()
                    {
                        std::fs::create_dir_all(p)?;
                    }
                    CmprssOutput::Path(p.clone())
                }
                CmprssOutput::Pipe(_) | CmprssOutput::Writer(_) => output,
            };
            last.extract(input, final_output)
        })
    }

    fn list(&self, input: CmprssInput) -> Result {
        debug_assert!(!self.compressors.is_empty(), "pipeline is never empty");
        if self.compressors.len() == 1 {
            return self.compressors[0].list(input);
        }
        // Same plumbing as `extract`, except the innermost compressor lists
        // its entries to stdout instead of unpacking. Outer layers still
        // decompress into the in-memory pipe so the innermost container sees
        // plain archive bytes.
        let stages = self
            .compressors
            .iter()
            .rev()
            .map(|c| c.clone_boxed())
            .collect();
        Self::run_threaded(stages, input, StageAction::Extract, |innermost, input| {
            innermost.list(input)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

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
    /// thread-dispatch in `Pipeline::compress`. Previously the pipeline
    /// reconstructed each stage from its *name* alone, producing a default
    /// Gzip regardless of the level the user requested.
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
}
