use assert_cmd::prelude::*;
use assert_fs::prelude::*;
use predicates::prelude::*;
use std::process::Command;

#[allow(dead_code)]
#[test]
fn tar_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let file = assert_fs::NamedTempFile::new("test.txt")?;
    file.write_str("garbage data for testing")?;
    let working_dir = assert_fs::TempDir::new()?;
    let archive = working_dir.child("archive.tar");
    archive.assert(predicate::path::missing());

    let mut compress = Command::cargo_bin("cmprss")?;
    compress.arg("tar").arg(file.path()).arg(archive.path());
    compress.assert().success();
    archive.assert(predicate::path::is_file());

    let mut extract = Command::cargo_bin("cmprss")?;
    extract
        .arg("tar")
        .arg("--extract")
        .arg("--input")
        .arg(archive.path())
        .arg("--output")
        .arg(working_dir.path());
    extract.assert().success();

    // Assert the files are identical
    working_dir
        .child("test.txt")
        .assert(predicate::path::eq_file(file.path()));

    Ok(())
}
