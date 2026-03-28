extern crate assert_cmd;
extern crate assert_fs;
extern crate predicates;

use assert_cmd::prelude::*;
use assert_fs::prelude::*;
use assert_fs::TempDir;
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

/// Full roundtrip: directory -> tar.xz -> directory
#[test]
fn test_tar_xz_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;

    let source_dir = temp_dir.child("source");
    source_dir.create_dir_all()?;
    let test_file = source_dir.child("data.txt");
    test_file.write_str("tar.xz roundtrip content")?;

    let archive = temp_dir.child("archive.tar.xz");
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
    extracted_file.assert(predicate::str::contains("tar.xz roundtrip content"));

    Ok(())
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
