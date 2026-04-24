use assert_cmd::prelude::*;
use assert_fs::prelude::*;
use predicates::prelude::*;
use std::process::Command;

mod common;
use common::*;

/// `cmprss tar --append` grows an existing tar archive with new entries and
/// the resulting archive extracts both original and appended files.
#[test]
fn tar_append_adds_entry() -> Result<(), Box<dyn std::error::Error>> {
    let original = create_test_file("original.txt", "original contents")?;
    let extra = create_test_file("extra.txt", "appended contents")?;
    let working_dir = create_working_dir()?;
    let archive = working_dir.child("archive.tar");

    Command::cargo_bin("cmprss")?
        .arg("tar")
        .arg(original.path())
        .arg(archive.path())
        .assert()
        .success();
    archive.assert(predicate::path::is_file());

    Command::cargo_bin("cmprss")?
        .arg("tar")
        .arg("--append")
        .arg(extra.path())
        .arg(archive.path())
        .assert()
        .success();

    Command::cargo_bin("cmprss")?
        .arg("tar")
        .arg("--extract")
        .arg(archive.path())
        .arg(working_dir.path())
        .assert()
        .success();

    assert_files_equal(original.path(), &working_dir.child("original.txt"));
    assert_files_equal(extra.path(), &working_dir.child("extra.txt"));
    Ok(())
}

/// `cmprss zip --append` grows an existing zip archive with new entries.
#[test]
fn zip_append_adds_entry() -> Result<(), Box<dyn std::error::Error>> {
    let original = create_test_file("original.txt", "original contents")?;
    let extra = create_test_file("extra.txt", "appended contents")?;
    let working_dir = create_working_dir()?;
    let archive = working_dir.child("archive.zip");

    Command::cargo_bin("cmprss")?
        .arg("zip")
        .arg(original.path())
        .arg(archive.path())
        .assert()
        .success();
    archive.assert(predicate::path::is_file());

    Command::cargo_bin("cmprss")?
        .arg("zip")
        .arg("--append")
        .arg(extra.path())
        .arg(archive.path())
        .assert()
        .success();

    Command::cargo_bin("cmprss")?
        .arg("zip")
        .arg("--extract")
        .arg(archive.path())
        .arg(working_dir.path())
        .assert()
        .success();

    assert_files_equal(original.path(), &working_dir.child("original.txt"));
    assert_files_equal(extra.path(), &working_dir.child("extra.txt"));
    Ok(())
}

/// Appending to a stream codec (e.g. gzip) must fail cleanly — there's no
/// notion of "entries" in a stream format.
#[test]
fn gzip_append_fails() -> Result<(), Box<dyn std::error::Error>> {
    let working_dir = create_working_dir()?;
    let archive = working_dir.child("data.gz");
    let input = create_test_file("data.txt", "hello")?;

    // Create a valid .gz to append to.
    Command::cargo_bin("cmprss")?
        .arg("gzip")
        .arg(input.path())
        .arg(archive.path())
        .assert()
        .success();

    let extra = create_test_file("extra.txt", "more")?;
    Command::cargo_bin("cmprss")?
        .arg("gzip")
        .arg("--append")
        .arg(extra.path())
        .arg(archive.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("do not support --append"));
    Ok(())
}

/// Appending to a compound pipeline like `tar.gz` must fail — it would
/// require decompressing and recompressing the whole archive.
#[test]
fn tar_gz_append_fails() -> Result<(), Box<dyn std::error::Error>> {
    let working_dir = create_working_dir()?;
    let archive = working_dir.child("archive.tar.gz");
    let input = create_test_file("data.txt", "hello")?;

    Command::cargo_bin("cmprss")?
        .arg("tar.gz")
        .arg(input.path())
        .arg(archive.path())
        .assert()
        .success();

    let extra = create_test_file("extra.txt", "more")?;
    Command::cargo_bin("cmprss")?
        .arg("tar.gz")
        .arg("--append")
        .arg(extra.path())
        .arg(archive.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("compound archive"));
    Ok(())
}

/// `--append` with a non-existent target must error rather than create a new
/// archive.
#[test]
fn tar_append_missing_target_errors() -> Result<(), Box<dyn std::error::Error>> {
    let working_dir = create_working_dir()?;
    let missing = working_dir.child("missing.tar");
    let extra = create_test_file("extra.txt", "x")?;

    // Non-existent trailing path is normally treated as the output to create;
    // --append should reject it instead.
    Command::cargo_bin("cmprss")?
        .arg("tar")
        .arg("--append")
        .arg(extra.path())
        .arg(missing.path())
        .assert()
        .failure();
    Ok(())
}
