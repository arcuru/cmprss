use crate::utils::*;
use std::io::{self, Read, Write};
use std::path::Path;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;

/// A compressor that chains multiple compressors together
/// This allows for multi-level compression formats like tar.gz
pub struct MultiLevelCompressor {
    // The chain of compressors to apply in order (innermost to outermost)
    compressors: Vec<Box<dyn Compressor>>,
}

impl MultiLevelCompressor {
    /// Create a new MultiLevelCompressor with a chain of compressors
    pub fn new(compressors: Vec<Box<dyn Compressor>>) -> Self {
        MultiLevelCompressor { compressors }
    }

    /// Create a new MultiLevelCompressor from compressor type names
    pub fn from_names(compressor_names: &[String]) -> io::Result<Self> {
        let mut compressors: Vec<Box<dyn Compressor>> = Vec::new();

        for name in compressor_names {
            let compressor: Box<dyn Compressor> = match name.as_str() {
                "tar" => Box::new(crate::backends::Tar::default()),
                "gzip" | "gz" => Box::new(crate::backends::Gzip::default()),
                "xz" => Box::new(crate::backends::Xz::default()),
                "bzip2" | "bz2" => Box::new(crate::backends::Bzip2::default()),
                "zip" => Box::new(crate::backends::Zip::default()),
                "zstd" | "zst" => Box::new(crate::backends::Zstd::default()),
                "lz4" => Box::new(crate::backends::Lz4::default()),
                _ => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        format!("Unknown compressor type: {}", name),
                    ))
                }
            };
            compressors.push(compressor);
        }

        Ok(Self { compressors })
    }

    /// Get a string representation of the chained format (e.g., "tar.gz")
    fn format_chain(&self) -> String {
        // Create a format string like "tar.gz" from the chain of compressors
        self.compressors
            .iter()
            .map(|c| c.extension())
            .rev() // Reverse to get innermost first
            .collect::<Vec<&str>>()
            .join(".")
    }

    /// Create a new compressor instance based on its name
    fn create_compressor(name: &str) -> io::Result<Box<dyn Compressor>> {
        match name {
            "tar" => Ok(Box::new(crate::backends::Tar::default())),
            "gzip" | "gz" => Ok(Box::new(crate::backends::Gzip::default())),
            "xz" => Ok(Box::new(crate::backends::Xz::default())),
            "bzip2" | "bz2" => Ok(Box::new(crate::backends::Bzip2::default())),
            "zip" => Ok(Box::new(crate::backends::Zip::default())),
            "zstd" | "zst" => Ok(Box::new(crate::backends::Zstd::default())),
            "lz4" => Ok(Box::new(crate::backends::Lz4::default())),
            _ => Err(io::Error::new(
                io::ErrorKind::Other,
                format!("Unknown compressor type: {}", name),
            )),
        }
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

impl Compressor for MultiLevelCompressor {
    fn name(&self) -> &str {
        // Return the name of the first (outermost) compressor
        if let Some(comp) = self.compressors.first() {
            comp.name()
        } else {
            "multi"
        }
    }

    fn extension(&self) -> &str {
        // This is a bit of a hack since we can't return an owned String from this method
        // We'll just return the extension of the outermost compressor
        if let Some(comp) = self.compressors.first() {
            comp.extension()
        } else {
            "multi"
        }
    }

    fn default_extracted_target(&self) -> ExtractedTarget {
        // The extracted target depends on the innermost compressor
        if let Some(comp) = self.compressors.last() {
            // The innermost compressor's target is what matters
            comp.default_extracted_target()
        } else {
            // If there are no compressors (shouldn't happen), default to FILE
            ExtractedTarget::FILE
        }
    }

    fn is_archive(&self, in_path: &Path) -> bool {
        // Check if the path matches our format chain
        if let Some(filename) = in_path.to_str() {
            let format_chain = self.format_chain();
            filename.ends_with(&format_chain)
        } else {
            false
        }
    }

    fn compress(&self, input: CmprssInput, output: CmprssOutput) -> Result<(), io::Error> {
        if self.compressors.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "No compressors in multi-level chain",
            ));
        }

        if self.compressors.len() == 1 {
            return self.compressors[0].compress(input, output);
        }

        let mut op_compressors: Vec<Box<dyn Compressor>> = self
            .compressors
            .iter()
            .map(|c| Self::create_compressor(c.name()).unwrap()) // TODO: Handle error properly
            .collect();

        let mut handles = Vec::new();
        let mut current_thread_input = input; // Consumed by the first (innermost) compressor
        let buffer_size = 64 * 1024;

        // Process all but the last (outermost) compressor in separate threads
        for i in 0..op_compressors.len() - 1 {
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
            handle.join().unwrap()?; // TODO: Handle thread errors properly
        }
        Ok(())
    }

    fn extract(&self, input: CmprssInput, output: CmprssOutput) -> Result<(), io::Error> {
        if self.compressors.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "No compressors in multi-level chain for extraction",
            ));
        }

        if self.compressors.len() == 1 {
            return self.compressors[0].extract(input, output);
        }

        let mut op_extractors: Vec<Box<dyn Compressor>> = self
            .compressors
            .iter()
            .rev() // Iterate from Outermost to Innermost
            .map(|c| Self::create_compressor(c.name()).unwrap()) // TODO: Handle error properly
            .collect();

        let mut handles = Vec::new();
        let mut current_thread_input = input; // Consumed by the first (outermost) extractor
        let buffer_size = 64 * 1024;

        // Process all but the last (innermost) extractor in separate threads.
        for i in 0..op_extractors.len() - 1 {
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
                if last_extractor.default_extracted_target() == ExtractedTarget::DIRECTORY {
                    if !p.exists() {
                        std::fs::create_dir_all(p)?;
                    }
                    // If it's a directory, the tar extractor (usually innermost) will handle it.
                    // The path provided should be the target directory.
                }
                // Always pass the path; the backend decides how to use it.
                CmprssOutput::Path(p.clone())
            }
            CmprssOutput::Pipe(_) => output,
            CmprssOutput::Writer(_) => output,
        };

        last_extractor.extract(current_thread_input, final_output)?;

        for handle in handles {
            handle.join().unwrap()?; // TODO: Handle thread errors properly
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::{Read, Write};
    use tempfile::tempdir;

    #[test]
    fn test_multi_level_compression() -> Result<(), io::Error> {
        // Create a temporary directory for our test
        let temp_dir = tempdir()?;

        // Create a test file
        let test_content = "This is a test file for multi-level compression";
        let test_file_path = temp_dir.path().join("test.txt");
        fs::write(&test_file_path, test_content)?;

        // Create a tar.gz compressor (tar first, then gzip)
        let compressors: Vec<Box<dyn Compressor>> = vec![
            Box::new(crate::backends::Tar::default()),
            Box::new(crate::backends::Gzip::default()),
        ];
        let multi_compressor = MultiLevelCompressor::new(compressors);

        // Compress the test file
        let archive_path = temp_dir.path().join("test.tar.gz");
        multi_compressor.compress(
            CmprssInput::Path(vec![test_file_path.clone()]),
            CmprssOutput::Path(archive_path.clone()),
        )?;

        // Verify the archive was created
        assert!(archive_path.exists());

        // Extract the archive
        let output_dir = temp_dir.path().join("extracted");
        fs::create_dir(&output_dir)?;
        multi_compressor.extract(
            CmprssInput::Path(vec![archive_path.clone()]),
            CmprssOutput::Path(output_dir.clone()),
        )?;

        // Verify the file was extracted correctly
        let extracted_file = output_dir.join("test.txt");
        assert!(extracted_file.exists());

        // Verify the content is the same
        let extracted_content = fs::read_to_string(extracted_file)?;
        assert_eq!(extracted_content, test_content);

        Ok(())
    }
}
