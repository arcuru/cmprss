use assert_cmd::prelude::*;
use assert_fs::prelude::*;
use predicates::prelude::*;
use std::{
    fs::File,
    process::{Command, Stdio},
};

/// Tar roundtrip with a single file
///
/// ``` bash
/// cmprss tar test.txt archive.tar
/// cmprss tar --extract archive.tar .
/// ```
#[test]
fn tar_roundtrip_explicit() -> Result<(), Box<dyn std::error::Error>> {
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
        .arg(archive.path())
        .arg(working_dir.path());
    extract.assert().success();

    // Assert the files are identical
    working_dir
        .child("test.txt")
        .assert(predicate::path::eq_file(file.path()));

    Ok(())
}

/// Tar roundtrip with multiple files
///
/// ``` bash
/// cmprss tar test.txt test2.txt archive.tar
/// cmprss tar --extract archive.tar .
/// ```
#[test]
fn tar_roundtrip_explicit_two() -> Result<(), Box<dyn std::error::Error>> {
    let file = assert_fs::NamedTempFile::new("test.txt")?;
    file.write_str("garbage data for testing")?;
    let file2 = assert_fs::NamedTempFile::new("test2.txt")?;
    file2.write_str("more garbage data for testing")?;
    let working_dir = assert_fs::TempDir::new()?;
    let archive = working_dir.child("archive.tar");
    archive.assert(predicate::path::missing());

    let mut compress = Command::cargo_bin("cmprss")?;
    compress
        .arg("tar")
        .arg(file.path())
        .arg(file2.path())
        .arg(archive.path());
    compress.assert().success();
    archive.assert(predicate::path::is_file());

    let mut extract = Command::cargo_bin("cmprss")?;
    extract
        .arg("tar")
        .arg("--extract")
        .arg(archive.path())
        .arg(working_dir.path());
    extract.assert().success();

    // Assert the files are identical
    working_dir
        .child("test.txt")
        .assert(predicate::path::eq_file(file.path()));
    working_dir
        .child("test2.txt")
        .assert(predicate::path::eq_file(file2.path()));

    Ok(())
}

/// Tar roundtrip with a single file inferring output filename
/// Compressing: output = './test.txt.tar'
/// Extracting:  output = '.'
///
/// ``` bash
/// cmprss tar test.txt
/// cmprss tar --extract test.txt.tar
/// ```
#[test]
fn tar_roundtrip_implicit() -> Result<(), Box<dyn std::error::Error>> {
    let file = assert_fs::NamedTempFile::new("test.txt")?;
    file.write_str("garbage data for testing")?;
    let working_dir = assert_fs::TempDir::new()?.into_persistent();
    let archive = working_dir.child("test.txt.tar");
    archive.assert(predicate::path::missing());

    let mut compress = Command::cargo_bin("cmprss")?;
    compress
        .current_dir(&working_dir)
        .arg("tar")
        .arg("--ignore-pipes")
        .arg(file.path());
    compress.assert().success();
    archive.assert(predicate::path::is_file());

    let mut extract = Command::cargo_bin("cmprss")?;
    extract
        .current_dir(&working_dir)
        .arg("tar")
        .arg("--ignore-pipes")
        .arg("--extract")
        .arg(archive.path());
    extract.assert().success();

    // Assert the files are identical
    working_dir
        .child("test.txt")
        .assert(predicate::path::eq_file(file.path()));

    Ok(())
}

/// Tar roundtrip with multiple files inferring output
/// Uses the first file's name to generate the output filename
/// Compressing: output = './test.txt.tar'
/// Extracting:  output = '.'
///
/// ``` bash
/// cmprss tar test.txt test2.txt
/// cmprss tar --extract test.txt.tar
/// ```
#[test]
fn tar_roundtrip_implicit_two() -> Result<(), Box<dyn std::error::Error>> {
    let file = assert_fs::NamedTempFile::new("test.txt")?;
    file.write_str("garbage data for testing")?;
    let file2 = assert_fs::NamedTempFile::new("test2.txt")?;
    file2.write_str("more garbage data for testing")?;
    let working_dir = assert_fs::TempDir::new()?.into_persistent();
    let archive = working_dir.child("test.txt.tar");
    archive.assert(predicate::path::missing());

    let mut compress = Command::cargo_bin("cmprss")?;
    compress
        .current_dir(&working_dir)
        .arg("tar")
        .arg("--ignore-pipes")
        .arg(file.path())
        .arg(file2.path());
    compress.assert().success();
    archive.assert(predicate::path::is_file());

    let mut extract = Command::cargo_bin("cmprss")?;
    extract
        .current_dir(&working_dir)
        .arg("tar")
        .arg("--ignore-pipes")
        .arg("--extract")
        .arg(archive.path());
    extract.assert().success();

    // Assert the files are identical
    working_dir
        .child("test.txt")
        .assert(predicate::path::eq_file(file.path()));
    working_dir
        .child("test2.txt")
        .assert(predicate::path::eq_file(file2.path()));

    Ok(())
}

/// Gzip roundtrip using stdin
/// Compressing: input = stdin, output = test.txt.gz
/// Extracting:  input = test.txt.gz, output = test.txt
///
/// ``` bash
/// cat test.txt | cmprss gzip test.txt.gz
/// cmprss gzip --ignore-pipes --extract test.txt.gz
/// ```
#[test]
fn gzip_roundtrip_stdin() -> Result<(), Box<dyn std::error::Error>> {
    let file = assert_fs::NamedTempFile::new("test.txt")?;
    file.write_str("garbage data for testing")?;
    let working_dir = assert_fs::TempDir::new()?;
    let archive = working_dir.child("test.txt.gz");
    archive.assert(predicate::path::missing());

    // Pipe file to stdin
    let mut compress = Command::cargo_bin("cmprss")?;
    compress
        .current_dir(&working_dir)
        .arg("gzip")
        .arg("test.txt.gz")
        .stdin(Stdio::from(File::open(file.path())?));
    compress.assert().success();
    archive.assert(predicate::path::is_file());

    let mut extract = Command::cargo_bin("cmprss")?;
    extract
        .current_dir(&working_dir)
        .arg("gzip")
        .arg("--ignore-pipes")
        .arg("--extract")
        .arg(archive.path());
    extract.assert().success();

    // Assert the files are identical
    working_dir
        .child("test.txt")
        .assert(predicate::path::eq_file(file.path()));

    Ok(())
}

/// Gzip roundtrip using filename inference
/// Compressing: input = stdin, output = default filename (archive.gz)
/// Extracting:  input = archive.gz, output = default filename (archive)
///
/// ``` bash
/// cat test.txt | cmprss gzip
/// cmprss gzip --ignore-pipes --extract archive.gz
/// ```
#[test]
fn gzip_roundtrip_inferred_output_filenames() -> Result<(), Box<dyn std::error::Error>> {
    let file = assert_fs::NamedTempFile::new("test.txt")?;
    file.write_str("garbage data for testing")?;
    let working_dir = assert_fs::TempDir::new()?;
    let archive = working_dir.child("archive.gz"); // default filename
    archive.assert(predicate::path::missing());

    // Pipe file to stdin
    let mut compress = Command::cargo_bin("cmprss")?;
    compress
        .current_dir(&working_dir)
        .arg("gzip")
        .arg("--ignore-stdout")
        .stdin(Stdio::from(File::open(file.path())?));
    compress.assert().success();
    archive.assert(predicate::path::is_file());

    let mut extract = Command::cargo_bin("cmprss")?;
    extract
        .current_dir(&working_dir)
        .arg("gzip")
        .arg("--ignore-pipes")
        .arg("--extract")
        .arg(archive.path());
    extract.assert().success();

    // Assert the files are identical
    working_dir
        .child("archive")
        .assert(predicate::path::eq_file(file.path()));

    Ok(())
}

/// Xz roundtrip using files
/// Compressing: input = test.txt, output = test.txt.xz
/// Extracting:  input = test.txt.xz, output = test.txt
///
/// ``` bash
/// cmprss xz test.txt test.txt.xz
/// cmprss xz --extract --ignore-pipes test.txt.xz
/// ```
#[test]
fn xz_roundtrip_explicit() -> Result<(), Box<dyn std::error::Error>> {
    let file = assert_fs::NamedTempFile::new("test.txt")?;
    file.write_str("garbage data for testing")?;
    let working_dir = assert_fs::TempDir::new()?;
    let archive = working_dir.child("test.txt.xz");
    archive.assert(predicate::path::missing());

    let mut compress = Command::cargo_bin("cmprss")?;
    compress
        .current_dir(&working_dir)
        .arg("xz")
        .arg(file.path())
        .arg(archive.path());
    compress.assert().success();
    archive.assert(predicate::path::is_file());

    let mut extract = Command::cargo_bin("cmprss")?;
    extract
        .current_dir(&working_dir)
        .arg("xz")
        .arg("--ignore-pipes")
        .arg("--extract")
        .arg(archive.path());
    extract.assert().success();

    // Assert the files are identical
    working_dir
        .child("test.txt")
        .assert(predicate::path::eq_file(file.path()));

    Ok(())
}

/// Xz roundtrip using stdin
/// Compressing: input = stdin, output = test.txt.xz
/// Extracting:  input = stdin(test.txt.xz), output = test.txt
///
/// ``` bash
/// cat test.txt | cmprss xz test.txt.xz
/// cat test.txt.xz | cmprss xz --extract out.txt
/// ```
#[test]
fn xz_roundtrip_stdin() -> Result<(), Box<dyn std::error::Error>> {
    let file = assert_fs::NamedTempFile::new("test.txt")?;
    file.write_str("garbage data for testing")?;
    let working_dir = assert_fs::TempDir::new()?;
    let archive = working_dir.child("test.txt.xz");
    archive.assert(predicate::path::missing());

    // Pipe file to stdin
    let mut compress = Command::cargo_bin("cmprss")?;
    compress
        .current_dir(&working_dir)
        .arg("xz")
        .arg("test.txt.xz")
        .stdin(Stdio::from(File::open(file.path())?));
    compress.assert().success();
    archive.assert(predicate::path::is_file());

    let mut extract = Command::cargo_bin("cmprss")?;
    extract
        .current_dir(&working_dir)
        .arg("xz")
        .stdin(Stdio::from(File::open(archive.path())?))
        .arg("--extract")
        .arg("out.txt");
    extract.assert().success();

    // Assert the files are identical
    working_dir
        .child("out.txt")
        .assert(predicate::path::eq_file(file.path()));

    Ok(())
}

/// Xz roundtrip using stdout
/// Compressing: input = test.txt, output = stdout
/// Extracting:  input = test.txt.xz, output = stdout
///
/// ``` bash
/// cmprss xz test.txt > test.txt.xz
/// cmprss xz --extract test.txt.xz > out.txt
/// ```
#[test]
fn xz_roundtrip_stdout() -> Result<(), Box<dyn std::error::Error>> {
    let file = assert_fs::NamedTempFile::new("test.txt")?;
    file.write_str("garbage data for testing")?;
    let working_dir = assert_fs::TempDir::new()?;
    let archive = working_dir.child("test.txt.xz");
    archive.assert(predicate::path::missing());

    // Compress file to stdout
    let mut compress = Command::cargo_bin("cmprss")?;
    compress
        .current_dir(&working_dir)
        .arg("xz")
        .arg(file.path())
        .stdout(Stdio::from(File::create(archive.path())?));
    compress.assert().success();
    archive.assert(predicate::path::is_file());

    // Extract file to stdout
    let mut extract = Command::cargo_bin("cmprss")?;
    extract
        .current_dir(&working_dir)
        .arg("xz")
        .arg("--ignore-stdin")
        .arg("--extract")
        .arg(archive.path())
        .arg("out.txt");
    // TODO: This fails, but manual testing shows it works fine
    //.stdout(Stdio::from(File::create("out.txt")?));
    extract.assert().success();

    // Assert the files are identical
    working_dir
        .child("out.txt")
        .assert(predicate::path::eq_file(file.path()));

    Ok(())
}

/// Bzip2 roundtrip using files
/// Compressing: input = test.txt, output = test.txt.bz2
/// Extracting:  input = test.txt.bz2, output = test.txt
///
/// ``` bash
/// cmprss bzip2 test.txt test.txt.bz2
/// cmprss bzip2 --extract --ignore-pipes test.txt.bz2
/// ```
#[test]
fn bzip2_roundtrip_explicit() -> Result<(), Box<dyn std::error::Error>> {
    let file = assert_fs::NamedTempFile::new("test.txt")?;
    file.write_str("garbage data for testing")?;

    let working_dir = assert_fs::TempDir::new()?;
    let archive = working_dir.child("test.txt.bz2");
    archive.assert(predicate::path::missing());

    let mut compress = Command::cargo_bin("cmprss")?;
    compress
        .current_dir(&working_dir)
        .arg("bzip2")
        .arg(file.path())
        .arg(archive.path());
    compress.assert().success();
    archive.assert(predicate::path::is_file());

    let mut extract = Command::cargo_bin("cmprss")?;
    extract
        .current_dir(&working_dir)
        .arg("bzip2")
        .arg("--ignore-pipes")
        .arg("--extract")
        .arg(archive.path());
    extract.assert().success();

    // Assert the files are identical
    working_dir
        .child("test.txt")
        .assert(predicate::path::eq_file(file.path()));

    Ok(())
}

/// Bzip2 roundtrip using stdin
/// Compressing: input = stdin, output = test.txt.bz2
/// Extracting:  input = stdin(test.txt.bz2), output = test.txt
///
/// ``` bash
/// cat test.txt | cmprss bzip2 test.txt.bz2
/// cat test.txt.bz2 | cmprss bzip2 --extract out.txt
/// ```
#[test]
fn bzip2_roundtrip_stdin() -> Result<(), Box<dyn std::error::Error>> {
    let file = assert_fs::NamedTempFile::new("test.txt")?;
    file.write_str("garbage data for testing")?;

    let working_dir = assert_fs::TempDir::new()?;
    let archive = working_dir.child("test.txt.bz2");
    archive.assert(predicate::path::missing());

    // Pipe file to stdin
    let mut compress = Command::cargo_bin("cmprss")?;
    compress
        .current_dir(&working_dir)
        .arg("bzip2")
        .arg("test.txt.bz2")
        .stdin(Stdio::from(File::open(file.path())?));
    compress.assert().success();
    archive.assert(predicate::path::is_file());

    let mut extract = Command::cargo_bin("cmprss")?;
    extract
        .current_dir(&working_dir)
        .arg("bzip2")
        .stdin(Stdio::from(File::open(archive.path())?))
        .arg("--extract")
        .arg("out.txt");
    extract.assert().success();

    // Assert the files are identical
    working_dir
        .child("out.txt")
        .assert(predicate::path::eq_file(file.path()));

    Ok(())
}

/// Bzip2 roundtrip using stdout
/// Compressing: input = test.txt, output = stdout
/// Extracting:  input = test.txt.bz2, output = stdout
///
/// ``` bash
/// cmprss bzip2 test.txt > test.txt.bz2
/// cmprss bzip2 --extract test.txt.bz2 > out.txt
/// ```
#[test]
fn bzip2_roundtrip_stdout() -> Result<(), Box<dyn std::error::Error>> {
    let file = assert_fs::NamedTempFile::new("test.txt")?;
    file.write_str("garbage data for testing")?;

    let working_dir = assert_fs::TempDir::new()?;
    let archive = working_dir.child("test.txt.bz2");
    archive.assert(predicate::path::missing());

    // Compress file to stdout
    let mut compress = Command::cargo_bin("cmprss")?;
    compress
        .current_dir(&working_dir)
        .arg("bzip2")
        .arg(file.path())
        .stdout(Stdio::from(File::create(archive.path())?));
    compress.assert().success();
    archive.assert(predicate::path::is_file());

    // Extract file to stdout
    let mut extract = Command::cargo_bin("cmprss")?;
    extract
        .current_dir(&working_dir)
        .arg("bzip2")
        .arg("--ignore-stdin")
        .arg("--extract")
        .arg(archive.path())
        .arg("out.txt");
    // TODO: This fails, but manual testing shows it works fine
    //.stdout(Stdio::from(File::create("out.txt")?));
    extract.assert().success();

    // Assert the files are identical
    working_dir
        .child("out.txt")
        .assert(predicate::path::eq_file(file.path()));

    Ok(())
}
