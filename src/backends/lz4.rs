use super::stream::{guard_file_output, open_input, open_output};
use crate::progress::{ProgressArgs, copy_with_progress};
use crate::utils::{CmprssInput, CmprssOutput, CommonArgs, Compressor, Result};
use clap::Args;
use lz4_flex::frame::{FrameDecoder, FrameEncoder};
use std::io;

#[derive(Args, Debug)]
pub struct Lz4Args {
    #[clap(flatten)]
    pub common_args: CommonArgs,

    #[clap(flatten)]
    pub progress_args: ProgressArgs,
}

#[derive(Default)]
pub struct Lz4 {
    pub progress_args: ProgressArgs,
}

impl Lz4 {
    pub fn new(args: &Lz4Args) -> Lz4 {
        Lz4 {
            progress_args: args.progress_args,
        }
    }
}

impl Compressor for Lz4 {
    /// The standard extension for the lz4 format.
    fn extension(&self) -> &str {
        "lz4"
    }

    /// Full name for lz4.
    fn name(&self) -> &str {
        "lz4"
    }

    /// Compress an input file or pipe to a lz4 archive
    fn compress(&self, input: CmprssInput, output: CmprssOutput) -> Result {
        guard_file_output(&output, "LZ4")?;
        let (mut input_stream, file_size) = open_input(input, "LZ4")?;

        if let CmprssOutput::Writer(writer) = output {
            let mut encoder = FrameEncoder::new(writer);
            io::copy(&mut input_stream, &mut encoder)?;
            encoder.finish()?;
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
            encoder.finish()?;
        }

        Ok(())
    }

    /// Extract a lz4 archive to an output file or pipe
    fn extract(&self, input: CmprssInput, output: CmprssOutput) -> Result {
        guard_file_output(&output, "LZ4")?;
        let (input_stream, file_size) = open_input(input, "LZ4")?;
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

    /// Test the basic interface of the Lz4 compressor
    #[test]
    fn test_lz4_interface() {
        let compressor = Lz4::default();
        test_compressor_interface(&compressor, "lz4", Some("lz4"));
    }

    /// Test the default compression level
    #[test]
    fn test_lz4_default_compression() -> Result {
        let compressor = Lz4::default();
        test_compression(&compressor)
    }
}
