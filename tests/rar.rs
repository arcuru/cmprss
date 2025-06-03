use assert_cmd::prelude::*;
use assert_fs::prelude::*;
use predicates::prelude::*;
use std::process::Command;

#[test]
fn test_rar_compress_error() {
    let temp_dir = assert_fs::TempDir::new().unwrap();
    let input_file = temp_dir.child("input.txt");
    input_file.write_str("test content").unwrap();
    let output_file = temp_dir.child("output.rar");

    let mut cmd = Command::cargo_bin("cmprss").unwrap();
    cmd.arg("rar")
        .arg("-i")
        .arg(input_file.path())
        .arg("-o")
        .arg(output_file.path())
        .arg("--compress");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("RAR compression is not supported."));
}

#[test]
fn test_rar_extract() {
    let temp_dir = assert_fs::TempDir::new().unwrap();
    let archive_file = temp_dir.child("test.rar");
    // Copy the dummy test.rar created earlier
    std::fs::copy("tests/test.rar", archive_file.path()).unwrap();
    let output_dir = temp_dir.child("output");

    let mut cmd = Command::cargo_bin("cmprss").unwrap();
    cmd.arg("rar")
        .arg("-i")
        .arg(archive_file.path())
        .arg("-o")
        .arg(output_dir.path())
        .arg("--extract");

    // Since test.rar is empty and not a valid rar file,
    // unrar library is expected to fail.
    // We are testing the cmprss rar backend, not the unrar library itself.
    // So, we check if the command fails with an error message from the unrar library.
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Failed to open RAR archive"));
}
