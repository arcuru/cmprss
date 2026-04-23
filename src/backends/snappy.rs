use super::stream::{guard_file_output, open_input, open_output};
use crate::progress::{ProgressArgs, copy_with_progress};
use crate::utils::{CmprssInput, CmprssOutput, CommonArgs, Compressor, Result};
use clap::Args;
use snap::read::FrameDecoder;
use snap::write::FrameEncoder;
use std::io::{self, Write};

#[derive(Args, Debug)]
pub struct SnappyArgs {
    #[clap(flatten)]
    pub common_args: CommonArgs,

    #[clap(flatten)]
    pub progress_args: ProgressArgs,
}

#[derive(Default)]
pub struct Snappy {
    pub progress_args: ProgressArgs,
}

impl Snappy {
    pub fn new(args: &SnappyArgs) -> Snappy {
        Snappy {
            progress_args: args.progress_args,
        }
    }
}

impl Compressor for Snappy {
    /// The standard extension for framed snappy files, per Google's reference
    /// implementation.
    fn extension(&self) -> &str {
        "sz"
    }

    /// Full name for snappy.
    fn name(&self) -> &str {
        "snappy"
    }

    /// Compress an input file or pipe to a snappy frame-format archive
    fn compress(&self, input: CmprssInput, output: CmprssOutput) -> Result {
        guard_file_output(&output, "Snappy")?;
        let (mut input_stream, file_size) = open_input(input, "Snappy")?;

        if let CmprssOutput::Writer(writer) = output {
            let mut encoder = FrameEncoder::new(writer);
            io::copy(&mut input_stream, &mut encoder)?;
            encoder.flush()?;
        } else {
            let output_stream = open_output(&output)?;
            let mut encoder = FrameEncoder::new(output_stream);
            copy_with_progress(
                &mut input_stream,
                &mut encoder,
                self.progress_args.chunk_size.size_in_bytes,
                file_size,
                self.progress_args.progress,
                &output,
            )?;
            encoder.flush()?;
        }

        Ok(())
    }

    /// Extract a snappy frame-format archive to an output file or pipe
    fn extract(&self, input: CmprssInput, output: CmprssOutput) -> Result {
        guard_file_output(&output, "Snappy")?;
        let (input_stream, file_size) = open_input(input, "Snappy")?;
        let mut decoder = FrameDecoder::new(input_stream);

        if let CmprssOutput::Writer(mut writer) = output {
            io::copy(&mut decoder, &mut writer)?;
        } else {
            let mut output_stream = open_output(&output)?;
            copy_with_progress(
                &mut decoder,
                &mut output_stream,
                self.progress_args.chunk_size.size_in_bytes,
                file_size,
                self.progress_args.progress,
                &output,
            )?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::*;

    /// Test the basic interface of the Snappy compressor
    #[test]
    fn test_snappy_interface() {
        let compressor = Snappy::default();
        test_compressor_interface(&compressor, "snappy", Some("sz"));
    }

    /// Test that the round-trip produces identical data
    #[test]
    fn test_snappy_default_compression() -> Result {
        let compressor = Snappy::default();
        test_compression(&compressor)
    }
}
