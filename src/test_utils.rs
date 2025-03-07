use crate::utils::ExtractedTarget;
use std::fs;
use std::io;
use std::path::Path;
use tempfile::tempdir;

use crate::utils::{CmprssInput, CmprssOutput, CompressionLevelValidator, Compressor};

/// Test basic trait functionality that should be common across all compressors
pub fn test_compressor_interface<T: Compressor>(
    compressor: &T,
    expected_name: &str,
    expected_extension: Option<&str>,
) {
    let ext = expected_extension.unwrap_or(expected_name);

    // Test name() returns expected value
    assert_eq!(compressor.name(), expected_name);

    // Test extension() returns expected value
    assert_eq!(compressor.extension(), ext);

    // Test is_archive() detection logic
    let temp_dir = tempdir().expect("Failed to create temp dir");

    // Test with matching extension
    let archive_path = temp_dir.path().join(format!("test.{}", ext));
    fs::File::create(&archive_path).expect("Failed to create test file");
    assert!(compressor.is_archive(&archive_path));

    // Test with non-matching extension
    let non_archive_path = temp_dir.path().join("test.txt");
    fs::File::create(&non_archive_path).expect("Failed to create test file");
    assert!(!compressor.is_archive(&non_archive_path));

    // Test default_compressed_filename
    let test_path = Path::new("test.txt");
    let expected = format!("test.txt.{}", ext);
    assert_eq!(compressor.default_compressed_filename(test_path), expected);

    // Test default_extracted_filename
    let formatted_name = format!("test.{}", ext);
    let archive_path = Path::new(&formatted_name);
    match compressor.default_extracted_target() {
        ExtractedTarget::FILE => {
            assert_eq!(compressor.default_extracted_filename(archive_path), "test");
        }
        ExtractedTarget::DIRECTORY => {
            assert_eq!(compressor.default_extracted_filename(archive_path), ".");
        }
    }

    // Test default_extracted_filename with non-matching extension
    let non_archive_path = Path::new("test.txt");
    match compressor.default_extracted_target() {
        ExtractedTarget::FILE => {
            assert_eq!(
                compressor.default_extracted_filename(non_archive_path),
                "archive"
            );
        }
        ExtractedTarget::DIRECTORY => {
            assert_eq!(compressor.default_extracted_filename(non_archive_path), ".");
        }
    }
}

/// Test compression and extraction functionality with a simple string
pub fn test_compressor_roundtrip<T: Compressor>(
    compressor: &T,
    test_data: &str,
) -> Result<(), io::Error> {
    let temp_dir = tempdir().expect("Failed to create temp dir");

    // Create test file
    let input_path = temp_dir.path().join("input.txt");
    fs::write(&input_path, test_data)?;

    // Compress
    let archive_path = temp_dir
        .path()
        .join(format!("archive.{}", compressor.extension()));
    compressor.compress(
        CmprssInput::Path(vec![input_path.clone()]),
        CmprssOutput::Path(archive_path.clone()),
    )?;

    // Extract
    let output_path = match compressor.default_extracted_target() {
        ExtractedTarget::FILE => temp_dir.path().join("output.txt"),
        ExtractedTarget::DIRECTORY => temp_dir.path().join("output"),
    };
    compressor.extract(
        CmprssInput::Path(vec![archive_path]),
        CmprssOutput::Path(output_path.clone()),
    )?;

    // Verify
    let input_filename = "input.txt";
    let output_data = match compressor.default_extracted_target() {
        ExtractedTarget::FILE => fs::read_to_string(output_path)?,
        ExtractedTarget::DIRECTORY => fs::read_to_string(output_path.join(input_filename))?,
    };
    assert_eq!(output_data, test_data);

    Ok(())
}

/// Test compression and extraction with different content sizes
pub fn test_compression<T: Compressor>(compressor: &T) -> Result<(), io::Error> {
    // Test with empty content
    test_compressor_roundtrip(compressor, "")?;

    // Test with small content
    test_compressor_roundtrip(compressor, "Small test content")?;

    // Test with medium content (generate a 10KB string)
    let medium_content = "0123456789".repeat(1024);
    test_compressor_roundtrip(compressor, &medium_content)?;

    Ok(())
}

/// Run a full suite of tests on a compressor implementation
pub fn run_compressor_tests<T: Compressor>(
    compressor: &T,
    expected_name: &str,
    expected_extension: Option<&str>,
) -> Result<(), io::Error> {
    // Test interface methods
    test_compressor_interface(compressor, expected_name, expected_extension);

    // Test compression/extraction functionality
    test_compression(compressor)?;

    Ok(())
}

/// Helper function to test CompressionValidator implementations
/// This avoids duplicating the same test pattern across multiple backends
pub fn test_compression_validator_helper<V: CompressionLevelValidator>(
    validator: &V,
    min_level: i32,
    max_level: i32,
    default_level: i32,
    fast_name_level: Option<i32>,
    best_name_level: Option<i32>,
    none_name_level: Option<i32>,
) {
    // Test range
    assert_eq!(validator.min_level(), min_level);
    assert_eq!(validator.max_level(), max_level);
    assert_eq!(validator.default_level(), default_level);

    // Test validation
    assert!(validator.is_valid_level(min_level));
    assert!(validator.is_valid_level(max_level));
    assert!(!validator.is_valid_level(min_level - 1));
    assert!(!validator.is_valid_level(max_level + 1));

    // Test middle level if range is big enough
    if max_level - min_level >= 2 {
        let mid_level = (min_level + max_level) / 2;
        assert!(validator.is_valid_level(mid_level));
    }

    // Test clamping
    assert_eq!(validator.validate_and_clamp_level(min_level - 1), min_level);
    assert_eq!(validator.validate_and_clamp_level(min_level), min_level);
    assert_eq!(validator.validate_and_clamp_level(max_level), max_level);
    assert_eq!(validator.validate_and_clamp_level(max_level + 1), max_level);

    // Test special names
    if let Some(level) = fast_name_level {
        assert_eq!(validator.name_to_level("fast"), Some(level));
    } else {
        assert_eq!(validator.name_to_level("fast"), None);
    }

    if let Some(level) = best_name_level {
        assert_eq!(validator.name_to_level("best"), Some(level));
    } else {
        assert_eq!(validator.name_to_level("best"), None);
    }

    if let Some(level) = none_name_level {
        assert_eq!(validator.name_to_level("none"), Some(level));
    } else {
        assert_eq!(validator.name_to_level("none"), None);
    }

    // Test invalid name
    assert_eq!(validator.name_to_level("invalid"), None);
}
