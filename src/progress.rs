use crate::utils::CmprssOutput;
use clap::Args;
use indicatif::{HumanBytes, ProgressBar};
use std::io::{self, Read, Write};
use std::str::FromStr;
use std::time::Duration;
use std::time::Instant;

#[derive(clap::ValueEnum, Clone, Copy, Debug, Default)]
pub enum ProgressDisplay {
    #[default]
    Auto,
    On,
    Off,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChunkSize {
    pub size_in_bytes: usize,
}

impl Default for ChunkSize {
    fn default() -> Self {
        ChunkSize {
            size_in_bytes: 8192,
        }
    }
}

impl FromStr for ChunkSize {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Try to parse s as just a number
        if let Ok(num) = s.parse::<usize>() {
            if num == 0 {
                return Err("Invalid number");
            }
            return Ok(ChunkSize { size_in_bytes: num });
        }
        // Simplify so that we always assume base 2, regardless of whether we see
        // 'kb' or 'kib'
        let mut s = s.to_lowercase();
        if s.ends_with("ib") {
            s.truncate(s.len() - 2);
            s.push('b');
        };
        let (num_str, unit) = s.split_at(s.len() - 2);
        let num = num_str.parse::<usize>().map_err(|_| "Invalid number")?;

        let size_in_bytes = match unit {
            "kb" => num * 1024,
            "mb" => num * 1024 * 1024,
            "gb" => num * 1024 * 1024 * 1024,
            _ => return Err("Invalid unit"),
        };
        if size_in_bytes == 0 {
            return Err("Invalid number");
        }

        Ok(ChunkSize { size_in_bytes })
    }
}

#[derive(Args, Debug, Default, Clone, Copy)]
pub struct ProgressArgs {
    /// Show progress.
    #[arg(long, value_enum, default_value = "auto")]
    pub progress: ProgressDisplay,

    /// Chunk size to use during the copy when showing the progress bar.
    #[arg(long, default_value = "8kib")]
    pub chunk_size: ChunkSize,
}

/// Create a progress bar if necessary based on settings
pub fn create_progress_bar(
    input_size: Option<u64>,
    progress: ProgressDisplay,
    output: &CmprssOutput,
) -> Option<ProgressBar> {
    match (progress, output) {
        (ProgressDisplay::Auto, CmprssOutput::Pipe(_)) => None,
        (ProgressDisplay::Off, _) => None,
        (_, _) => {
            let bar = match input_size {
                Some(size) => ProgressBar::new(size),
                None => ProgressBar::new_spinner(),
            };
            bar.set_style(
                indicatif::ProgressStyle::default_bar()
                    .template("{spinner:.green} [{elapsed_precise}] ({eta}) [{bar:40.cyan/blue}] {bytes}/{total_bytes} => {msg}").unwrap()
                    .progress_chars("#>-"),
            );
            bar.enable_steady_tick(Duration::from_millis(100));
            Some(bar)
        }
    }
}

/// A reader that tracks progress of bytes read
pub struct ProgressReader<R> {
    inner: R,
    bar: Option<ProgressBar>,
    total_read: u64,
    last_update: Instant,
    bytes_since_update: u64,
    bytes_per_update: u64,
}

impl<R: Read> ProgressReader<R> {
    pub fn new(inner: R, bar: Option<ProgressBar>) -> Self {
        ProgressReader {
            inner,
            bar,
            total_read: 0,
            last_update: Instant::now(),
            bytes_since_update: 0,
            bytes_per_update: 8192, // Start with 8KB, will adjust dynamically
        }
    }

    /// Updates the progress bar if enough bytes have been read since the last update.
    /// Dynamically adjusts the update frequency to target ~100ms between updates by
    /// tracking the elapsed time and adjusting bytes_per_update accordingly.
    fn maybe_update_progress(&mut self, bytes_read: u64) {
        if let Some(ref bar) = self.bar {
            self.bytes_since_update += bytes_read;

            if self.bytes_since_update >= self.bytes_per_update {
                let now = Instant::now();
                let elapsed = now.duration_since(self.last_update);

                // Update the progress
                bar.set_position(self.total_read);

                // Adjust bytes_per_update to target ~100ms between updates
                if elapsed < Duration::from_millis(50) {
                    self.bytes_per_update *= 2;
                } else if elapsed > Duration::from_millis(150) {
                    self.bytes_per_update = (self.bytes_per_update / 2).max(1024);
                }

                self.last_update = now;
                self.bytes_since_update = 0;
            }
        }
    }
}

impl<R: Read> Read for ProgressReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let bytes_read = self.inner.read(buf)?;
        if bytes_read > 0 {
            self.total_read += bytes_read as u64;
            self.maybe_update_progress(bytes_read as u64);
        }
        Ok(bytes_read)
    }
}

/// A writer that tracks progress of bytes written
pub struct ProgressWriter<W> {
    inner: W,
    bar: Option<ProgressBar>,
    total_written: u64,
    last_update: Instant,
    bytes_since_update: u64,
    bytes_per_update: u64,
}

impl<W: Write> ProgressWriter<W> {
    pub fn new(inner: W, bar: Option<ProgressBar>) -> Self {
        ProgressWriter {
            inner,
            bar,
            total_written: 0,
            last_update: Instant::now(),
            bytes_since_update: 0,
            bytes_per_update: 8192, // Start with 8KB, will adjust dynamically
        }
    }

    pub fn finish(self) {
        if let Some(bar) = self.bar {
            bar.finish();
        }
    }

    /// Updates the progress bar if enough bytes have been written since the last update.
    /// Dynamically adjusts the update frequency to target ~100ms between updates by
    /// tracking the elapsed time and adjusting bytes_per_update accordingly.
    fn maybe_update_progress(&mut self, bytes_written: u64) {
        if let Some(ref bar) = self.bar {
            self.bytes_since_update += bytes_written;

            if self.bytes_since_update >= self.bytes_per_update {
                let now = Instant::now();
                let elapsed = now.duration_since(self.last_update);

                // Update the progress
                bar.set_message(HumanBytes(self.total_written).to_string());

                // Adjust bytes_per_update to target ~100ms between updates
                if elapsed < Duration::from_millis(50) {
                    self.bytes_per_update *= 2;
                } else if elapsed > Duration::from_millis(150) {
                    self.bytes_per_update = (self.bytes_per_update / 2).max(1024);
                }

                self.last_update = now;
                self.bytes_since_update = 0;
            }
        }
    }
}

impl<W: Write> Write for ProgressWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let bytes_written = self.inner.write(buf)?;
        if bytes_written > 0 {
            self.total_written += bytes_written as u64;
            self.maybe_update_progress(bytes_written as u64);
        }
        Ok(bytes_written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

/// Process data with progress bar updates
pub fn copy_with_progress<R: Read, W: Write>(
    reader: R,
    writer: W,
    chunk_size: usize,
    input_size: Option<u64>,
    progress_display: ProgressDisplay,
    output: &CmprssOutput,
) -> io::Result<()> {
    // Create the progress bar if needed
    let progress_bar = create_progress_bar(input_size, progress_display, output);

    // Create reader and writer with progress tracking
    let mut reader = ProgressReader::new(reader, progress_bar.clone());
    let mut writer = ProgressWriter::new(writer, progress_bar);

    let mut buffer = vec![0; chunk_size];
    loop {
        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        writer.write_all(&buffer[..bytes_read])?;
    }
    writer.flush()?;
    writer.finish();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_size_parsing() {
        assert!(ChunkSize::from_str("0").is_err());
        assert!(ChunkSize::from_str("0mb").is_err());
        assert_eq!(
            ChunkSize::from_str("1").unwrap(),
            ChunkSize { size_in_bytes: 1 }
        );
        assert_eq!(
            ChunkSize::from_str("1kb").unwrap(),
            ChunkSize {
                size_in_bytes: 1024
            }
        );
        assert_eq!(
            ChunkSize::from_str("16kib").unwrap(),
            ChunkSize {
                size_in_bytes: 16 * 1024
            }
        );
        assert_eq!(
            ChunkSize::from_str("8mib").unwrap(),
            ChunkSize {
                size_in_bytes: 8 * 1024 * 1024
            }
        );
        assert_eq!(
            ChunkSize::from_str("16mb").unwrap(),
            ChunkSize {
                size_in_bytes: 16 * 1024 * 1024
            }
        );
        assert_eq!(
            ChunkSize::from_str("1gb").unwrap(),
            ChunkSize {
                size_in_bytes: 1024 * 1024 * 1024
            }
        );
        assert_eq!(
            ChunkSize::from_str("16gib").unwrap(),
            ChunkSize {
                size_in_bytes: 16 * 1024 * 1024 * 1024
            }
        );
    }
}
