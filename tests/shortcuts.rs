use assert_cmd::prelude::*;
use assert_fs::prelude::*;
use predicates::prelude::*;
use std::process::Command;

mod common;
use common::*;

/// Roundtrip helper: compress two files into `archive.<ext>` and extract into
/// a fresh directory, asserting that the original files come back identical.
fn shortcut_roundtrip(ext: &str) -> Result<(), Box<dyn std::error::Error>> {
    let file = create_test_file("test.txt", "garbage data for testing")?;
    let file2 = create_test_file("test2.txt", "more garbage data for testing")?;
    let working_dir = create_working_dir()?;
    let archive_name = format!("archive.{ext}");
    let archive = working_dir.child(&archive_name);
    archive.assert(predicate::path::missing());

    let mut compress = Command::cargo_bin("cmprss")?;
    compress
        .current_dir(&working_dir)
        .arg("--ignore-pipes")
        .arg(file.path())
        .arg(file2.path())
        .arg(&archive_name);
    compress.assert().success();
    archive.assert(predicate::path::is_file());

    let extract_dir = create_working_dir()?;
    let mut extract = Command::cargo_bin("cmprss")?;
    extract
        .current_dir(&extract_dir)
        .arg("--ignore-pipes")
        .arg(archive.path());
    extract.assert().success();

    assert_files_equal(file.path(), &extract_dir.child("test.txt"));
    assert_files_equal(file2.path(), &extract_dir.child("test2.txt"));

    Ok(())
}

mod shortcuts {
    use super::*;

    #[test]
    fn tgz() -> Result<(), Box<dyn std::error::Error>> {
        shortcut_roundtrip("tgz")
    }

    #[test]
    fn tbz() -> Result<(), Box<dyn std::error::Error>> {
        shortcut_roundtrip("tbz")
    }

    #[test]
    fn tbz2() -> Result<(), Box<dyn std::error::Error>> {
        shortcut_roundtrip("tbz2")
    }

    #[test]
    fn txz() -> Result<(), Box<dyn std::error::Error>> {
        shortcut_roundtrip("txz")
    }

    #[test]
    fn tzst() -> Result<(), Box<dyn std::error::Error>> {
        shortcut_roundtrip("tzst")
    }
}
