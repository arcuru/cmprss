use crate::utils::*;
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

impl Pipeline {
    /// Create a new Pipeline with the given compressors
    pub fn new(compressors: Vec<Box<dyn Compressor>>) -> Self {
        Pipeline { compressors }
    }

    /// Create a new Pipeline from compressor type names
    pub fn from_names(compressor_names: &[String]) -> Result<Self> {
        let compressors = compressor_names
            .iter()
            .map(|name| Self::create_compressor(name))
            .collect::<Result<Vec<_>>>()?;
        Ok(Self { compressors })
    }

    /// Get a string representation of the chained format (e.g., "tar.gz")
    fn format_chain(&self) -> String {
        self.compressors
            .iter()
            .map(|c| c.extension())
            .collect::<Vec<&str>>()
            .join(".")
    }

    fn create_compressor(name: &str) -> Result<Box<dyn Compressor>> {
        crate::backends::compressor_from_str(name)
            .ok_or_else(|| anyhow!("Unknown compressor type: {}", name))
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

        let mut op_compressors: Vec<Box<dyn Compressor>> = self
            .compressors
            .iter()
            .map(|c| Self::create_compressor(c.name()))
            .collect::<Result<Vec<_>>>()?;

        let mut handles = Vec::new();
        let mut current_thread_input = input; // Consumed by the first (innermost) compressor
        let buffer_size = 64 * 1024;

        // Process all but the last (outermost) compressor in separate threads
        for _ in 0..op_compressors.len() - 1 {
            let compressor_for_this_stage = op_compressors.remove(0);
            let (sender, receiver) = channel::<Vec<u8>>();
            let pipe_writer = PipeWriter::new(sender, buffer_size);
            let input_for_next_stage =
                CmprssInput::Reader(ReadWrapper(Box::new(PipeReader::new(receiver))));

            let actual_input_for_thread = current_thread_input; // Move current input to thread
            current_thread_input = input_for_next_stage; // Set up input for the *next* stage or final compressor

            let handle = thread::spawn(move || {
                compressor_for_this_stage.compress(
                    actual_input_for_thread,
                    CmprssOutput::Writer(WriteWrapper(Box::new(pipe_writer))),
                )
            });
            handles.push(handle);
        }

        // The last (outermost) compressor runs in the current thread and writes to the final output
        let last_compressor = op_compressors.remove(0);
        // current_thread_input here is the Reader from the penultimate stage
        last_compressor.compress(current_thread_input, output)?;

        for handle in handles {
            handle
                .join()
                .map_err(|_| anyhow!("Compression thread panicked"))??;
        }
        Ok(())
    }

    fn extract(&self, input: CmprssInput, output: CmprssOutput) -> Result {
        debug_assert!(!self.compressors.is_empty(), "pipeline is never empty");

        if self.compressors.len() == 1 {
            return self.compressors[0].extract(input, output);
        }

        let mut op_extractors: Vec<Box<dyn Compressor>> = self
            .compressors
            .iter()
            .rev()
            .map(|c| Self::create_compressor(c.name()))
            .collect::<Result<Vec<_>>>()?;

        let mut handles = Vec::new();
        let mut current_thread_input = input; // Consumed by the first (outermost) extractor
        let buffer_size = 64 * 1024;

        // Process all but the last (innermost) extractor in separate threads.
        for _ in 0..op_extractors.len() - 1 {
            let extractor_for_this_stage = op_extractors.remove(0);
            let (sender, receiver) = channel::<Vec<u8>>();
            let pipe_writer = PipeWriter::new(sender, buffer_size);
            let intermediate_output_for_thread =
                CmprssOutput::Writer(WriteWrapper(Box::new(pipe_writer)));
            let input_for_next_stage =
                CmprssInput::Reader(ReadWrapper(Box::new(PipeReader::new(receiver))));

            let actual_input_for_thread = current_thread_input; // Move current input to thread
            current_thread_input = input_for_next_stage; // Set up input for the *next* stage or final extractor

            let handle = thread::spawn(move || {
                extractor_for_this_stage
                    .extract(actual_input_for_thread, intermediate_output_for_thread)
            });
            handles.push(handle);
        }

        // The last (innermost) extractor runs in the current thread and writes to the final output
        let last_extractor = op_extractors.remove(0);
        // current_thread_input here is the Reader from the penultimate stage

        let final_output = match output {
            CmprssOutput::Path(ref p) => {
                if last_extractor.default_extracted_target() == ExtractedTarget::Directory
                    && !p.exists()
                {
                    std::fs::create_dir_all(p)?;
                }
                // If it's a directory, the tar extractor (usually innermost) will handle it.
                // The path provided should be the target directory.
                // Always pass the path; the backend decides how to use it.
                CmprssOutput::Path(p.clone())
            }
            CmprssOutput::Pipe(_) => output,
            CmprssOutput::Writer(_) => output,
        };

        last_extractor.extract(current_thread_input, final_output)?;

        for handle in handles {
            handle
                .join()
                .map_err(|_| anyhow!("Extraction thread panicked"))??;
        }
        Ok(())
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
}
