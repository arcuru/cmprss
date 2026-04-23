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

/// Roundtrip helper: pass `format` as the leading positional (e.g.
/// `cmprss tar.gz src/ out.tar.gz`), then extract with the same format
/// and compare contents.
fn format_prefix_roundtrip(
    format: &str,
    archive_ext: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let file = create_test_file("test.txt", "garbage data for testing")?;
    let file2 = create_test_file("test2.txt", "more garbage data for testing")?;
    let working_dir = create_working_dir()?;
    let archive_name = format!("archive.{archive_ext}");
    let archive = working_dir.child(&archive_name);
    archive.assert(predicate::path::missing());

    let mut compress = Command::cargo_bin("cmprss")?;
    compress
        .current_dir(&working_dir)
        .arg("--ignore-pipes")
        .arg(format)
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
        .arg(format)
        .arg("--extract")
        .arg(archive.path());
    extract.assert().success();

    assert_files_equal(file.path(), &extract_dir.child("test.txt"));
    assert_files_equal(file2.path(), &extract_dir.child("test2.txt"));

    Ok(())
}

mod format_prefix {
    use super::*;

    #[test]
    fn tar_gz() -> Result<(), Box<dyn std::error::Error>> {
        format_prefix_roundtrip("tar.gz", "tar.gz")
    }

    #[test]
    fn tgz() -> Result<(), Box<dyn std::error::Error>> {
        format_prefix_roundtrip("tgz", "tgz")
    }

    #[test]
    fn tar_xz() -> Result<(), Box<dyn std::error::Error>> {
        format_prefix_roundtrip("tar.xz", "tar.xz")
    }

    #[test]
    fn txz() -> Result<(), Box<dyn std::error::Error>> {
        format_prefix_roundtrip("txz", "txz")
    }

    #[test]
    fn tar_bz2() -> Result<(), Box<dyn std::error::Error>> {
        format_prefix_roundtrip("tar.bz2", "tar.bz2")
    }

    #[test]
    fn tbz2() -> Result<(), Box<dyn std::error::Error>> {
        format_prefix_roundtrip("tbz2", "tbz2")
    }

    #[test]
    fn tar_zst() -> Result<(), Box<dyn std::error::Error>> {
        format_prefix_roundtrip("tar.zst", "tar.zst")
    }

    #[test]
    fn tzst() -> Result<(), Box<dyn std::error::Error>> {
        format_prefix_roundtrip("tzst", "tzst")
    }
}
