use crate::utils::CmprssOutput;
use clap::Args;
use indicatif::{HumanBytes, ProgressBar};
use std::str::FromStr;

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

/// Progress bar for the compress process
pub struct Progress {
    /// The progress bar
    bar: ProgressBar,
    /// The number of bytes read from the input
    input_read: u64,
    /// The number of bytes written to the output
    output_written: u64,
}

/// Create a progress bar if necessary
pub fn progress_bar(
    input_size: Option<u64>,
    progress: ProgressDisplay,
    output: &CmprssOutput,
) -> Option<Progress> {
    match (progress, output) {
        (ProgressDisplay::Auto, CmprssOutput::Pipe(_)) => None,
        (ProgressDisplay::Off, _) => None,
        (_, _) => Some(Progress::new(input_size)),
    }
}

impl Progress {
    /// Create a new progress bar
    /// Draws to stderr by default
    pub fn new(input_size: Option<u64>) -> Self {
        let bar = match input_size {
            Some(size) => ProgressBar::new(size),
            None => ProgressBar::new_spinner(),
        };
        bar.set_style(
            indicatif::ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] ({eta}) [{bar:40.cyan/blue}] {bytes}/{total_bytes} => {msg}").unwrap()
                .progress_chars("#>-"),
        );
        Progress {
            bar,
            input_read: 0,
            output_written: 0,
        }
    }

    /// Update the progress bar with the number of bytes read from the input
    pub fn update_input(&mut self, bytes_read: u64) {
        self.input_read = bytes_read;
        self.bar.set_position(self.input_read);
    }

    /// Update the progress bar with the number of bytes written to the output
    pub fn update_output(&mut self, bytes_written: u64) {
        self.output_written = bytes_written;
        self.bar
            .set_message(HumanBytes(self.output_written).to_string());
    }

    /// Finish the progress bar
    pub fn finish(&self) {
        self.bar.finish();
    }
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
