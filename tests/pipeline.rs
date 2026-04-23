extern crate assert_cmd;
extern crate assert_fs;
extern crate predicates;

use assert_cmd::prelude::*;
use assert_fs::TempDir;
use assert_fs::prelude::*;
use predicates::prelude::*;
use std::process::Command;

// Test manual step-by-step tar.gz roundtrip
#[test]
fn test_tar_gz_manual_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let file = temp_dir.child("test.txt");
    file.write_str("test content")?;

    // Step 1: Create a tar file
    let tar_file = temp_dir.child("test.tar");
    Command::cargo_bin("cmprss")?
        .arg("tar")
        .arg(file.path())
        .arg(tar_file.path())
        .assert()
        .success();

    // Step 2: Compress the tar file with gzip
    let tar_gz_file = temp_dir.child("test.tar.gz");
    Command::cargo_bin("cmprss")?
        .arg("gzip")
        .arg(tar_file.path())
        .arg(tar_gz_file.path())
        .assert()
        .success();

    // Step 3: Extract the gzip layer
    let extract_tar = temp_dir.child("extracted.tar");
    Command::cargo_bin("cmprss")?
        .arg("gzip")
        .arg("--extract")
        .arg(tar_gz_file.path())
        .arg(extract_tar.path())
        .assert()
        .success();

    // Step 4: Extract the tar layer to a directory
    let extract_dir = temp_dir.child("extracted");
    extract_dir.create_dir_all()?;
    Command::cargo_bin("cmprss")?
        .arg("tar")
        .arg("--extract")
        .arg(extract_tar.path())
        .arg(extract_dir.path())
        .assert()
        .success();

    // Verify the extracted content
    let extracted_file = extract_dir.child("test.txt");
    extracted_file.assert(predicate::path::exists());
    extracted_file.assert(predicate::str::contains("test content"));

    Ok(())
}

// Test pipeline compression using tar.gz format
#[test]
fn test_tar_gz_compress() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;

    // Create a file structure for testing
    let source_dir = temp_dir.child("source");
    source_dir.create_dir_all()?;

    let test_file = source_dir.child("test_file.txt");
    test_file.write_str("test content for tar.gz compression")?;

    // Create a tar.gz archive directly in one step
    let archive = temp_dir.child("direct.tar.gz");
    Command::cargo_bin("cmprss")?
        .arg("--compress") // explicitly specify compression
        .arg(source_dir.path())
        .arg(archive.path())
        .assert()
        .success();

    // Verify the archive was created
    archive.assert(predicate::path::exists());

    Ok(())
}

// Test pipeline extraction using tar.gz format
#[test]
fn test_tar_gz_extract() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;

    // Create a file structure for testing
    let source_dir = temp_dir.child("source");
    source_dir.create_dir_all()?;

    let test_file = source_dir.child("test_file.txt");
    test_file.write_str("test content for tar.gz extraction")?;

    // Create a tar file first
    let tar_file = temp_dir.child("archive.tar");
    Command::cargo_bin("cmprss")?
        .arg("tar")
        .arg(source_dir.path())
        .arg(tar_file.path())
        .assert()
        .success();

    // Compress the tar with gzip
    let tar_gz_file = temp_dir.child("archive.tar.gz");
    Command::cargo_bin("cmprss")?
        .arg("gzip")
        .arg(tar_file.path())
        .arg(tar_gz_file.path())
        .assert()
        .success();

    // Create an extraction directory
    let extract_dir = temp_dir.child("extract");
    extract_dir.create_dir_all()?;

    // Extract the tar.gz archive
    Command::cargo_bin("cmprss")?
        .arg("--extract")
        .arg(tar_gz_file.path())
        .arg(extract_dir.path())
        .assert()
        .success();

    // Verify the file was extracted correctly (tar preserves the directory structure)
    let extracted_file = extract_dir.child("source").child("test_file.txt");
    extracted_file.assert(predicate::path::exists());
    extracted_file.assert(predicate::str::contains(
        "test content for tar.gz extraction",
    ));

    Ok(())
}

/// Full roundtrip for a `tar.<codec>` compound extension using cmprss alone.
/// Verifies that the extension resolver, compress pipeline, and extract
/// pipeline agree for the given compound format.
fn tar_pipeline_roundtrip(ext: &str) -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;

    let source_dir = temp_dir.child("source");
    source_dir.create_dir_all()?;
    let test_file = source_dir.child("data.txt");
    let content = format!("{ext} roundtrip content");
    test_file.write_str(&content)?;

    let archive = temp_dir.child(format!("archive.{ext}"));
    Command::cargo_bin("cmprss")?
        .arg("--compress")
        .arg(source_dir.path())
        .arg(archive.path())
        .assert()
        .success();

    let extract_dir = temp_dir.child("extract");
    extract_dir.create_dir_all()?;
    Command::cargo_bin("cmprss")?
        .arg("--extract")
        .arg(archive.path())
        .arg(extract_dir.path())
        .assert()
        .success();

    let extracted_file = extract_dir.child("source").child("data.txt");
    extracted_file.assert(predicate::path::exists());
    extracted_file.assert(predicate::str::contains(content));

    Ok(())
}

#[test]
fn test_tar_xz_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    tar_pipeline_roundtrip("tar.xz")
}

#[test]
fn test_tar_bz2_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    tar_pipeline_roundtrip("tar.bz2")
}

#[test]
fn test_tar_zst_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    tar_pipeline_roundtrip("tar.zst")
}

#[test]
fn test_tar_lzma_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    tar_pipeline_roundtrip("tar.lzma")
}

#[test]
fn test_tar_br_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    tar_pipeline_roundtrip("tar.br")
}

#[test]
fn test_tar_lz4_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    tar_pipeline_roundtrip("tar.lz4")
}

#[test]
fn test_tar_sz_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    tar_pipeline_roundtrip("tar.sz")
}

// Test pipeline extraction using tar.gz with explicit compress then auto-detect extract
#[test]
fn test_tar_gz_explicit_then_extract() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;

    // Create a simple test file
    let test_file = temp_dir.child("test.txt");
    test_file.write_str("test content for tar.gz")?;

    // Create a tar archive first (explicit command)
    let tar_file = temp_dir.child("test.tar");
    Command::cargo_bin("cmprss")?
        .arg("tar")
        .arg(test_file.path())
        .arg(tar_file.path())
        .assert()
        .success();

    // Compress the tar with gzip (explicit command)
    let tar_gz_file = temp_dir.child("test.tar.gz");
    Command::cargo_bin("cmprss")?
        .arg("gzip")
        .arg(tar_file.path())
        .arg(tar_gz_file.path())
        .assert()
        .success();

    // Create an extraction directory
    let extract_dir = temp_dir.child("extract");
    extract_dir.create_dir_all()?;

    // Extract using the tar.gz auto-detection
    Command::cargo_bin("cmprss")?
        .arg("-e") // Use short form for extract
        .arg(tar_gz_file.path())
        .arg(extract_dir.path())
        .assert()
        .success();

    // Verify the file was extracted correctly
    let extracted_file = extract_dir.child("test.txt");
    extracted_file.assert(predicate::path::exists());
    extracted_file.assert(predicate::str::contains("test content for tar.gz"));

    Ok(())
}
