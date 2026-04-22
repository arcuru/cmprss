use assert_cmd::prelude::*;
use assert_fs::prelude::*;
use predicates::prelude::*;
use std::process::Command;

mod common;
use common::*;

mod list {
    use super::*;

    /// `cmprss --list archive.tar` prints every entry's path, one per line.
    #[test]
    fn tar_archive() -> Result<(), Box<dyn std::error::Error>> {
        let working_dir = create_working_dir()?;
        let a = working_dir.child("alpha.txt");
        a.write_str("alpha")?;
        let b = working_dir.child("beta.txt");
        b.write_str("beta")?;

        let mut pack = Command::cargo_bin("cmprss")?;
        pack.current_dir(&working_dir)
            .args(["tar", "alpha.txt", "beta.txt", "out.tar"]);
        pack.assert().success();

        let mut list = Command::cargo_bin("cmprss")?;
        list.current_dir(&working_dir).args(["--list", "out.tar"]);
        list.assert()
            .success()
            .stdout(predicate::str::contains("alpha.txt"))
            .stdout(predicate::str::contains("beta.txt"));
        Ok(())
    }

    /// `cmprss --list archive.zip` enumerates file names via ZipArchive.
    #[test]
    fn zip_archive() -> Result<(), Box<dyn std::error::Error>> {
        let working_dir = create_working_dir()?;
        let a = working_dir.child("first.txt");
        a.write_str("first")?;
        let b = working_dir.child("second.txt");
        b.write_str("second")?;

        let mut pack = Command::cargo_bin("cmprss")?;
        pack.current_dir(&working_dir)
            .args(["zip", "first.txt", "second.txt", "out.zip"]);
        pack.assert().success();

        let mut list = Command::cargo_bin("cmprss")?;
        list.current_dir(&working_dir).args(["--list", "out.zip"]);
        list.assert()
            .success()
            .stdout(predicate::str::contains("first.txt"))
            .stdout(predicate::str::contains("second.txt"));
        Ok(())
    }

    /// Pipelines whose innermost layer is a container format list through
    /// the in-memory pipe plumbing shared with extract.
    #[test]
    fn tar_gz_archive() -> Result<(), Box<dyn std::error::Error>> {
        let working_dir = create_working_dir()?;
        let a = working_dir.child("inside.txt");
        a.write_str("inside")?;

        let mut pack = Command::cargo_bin("cmprss")?;
        pack.current_dir(&working_dir)
            .args(["tar", "inside.txt", "out.tar"]);
        pack.assert().success();

        let mut gz = Command::cargo_bin("cmprss")?;
        gz.current_dir(&working_dir)
            .args(["gzip", "out.tar", "out.tar.gz"]);
        gz.assert().success();

        let mut list = Command::cargo_bin("cmprss")?;
        list.current_dir(&working_dir)
            .args(["--list", "out.tar.gz"]);
        list.assert()
            .success()
            .stdout(predicate::str::contains("inside.txt"));
        Ok(())
    }

    /// Stream codecs (gzip, xz, …) don't carry multiple entries — listing
    /// must fail loudly rather than silently doing nothing.
    #[test]
    fn gzip_stream_rejects_list() -> Result<(), Box<dyn std::error::Error>> {
        let working_dir = create_working_dir()?;
        let a = working_dir.child("doc.txt");
        a.write_str("doc")?;

        let mut pack = Command::cargo_bin("cmprss")?;
        pack.current_dir(&working_dir)
            .args(["gzip", "doc.txt", "doc.txt.gz"]);
        pack.assert().success();

        let mut list = Command::cargo_bin("cmprss")?;
        list.current_dir(&working_dir)
            .args(["--list", "doc.txt.gz"]);
        list.assert()
            .failure()
            .stderr(predicate::str::contains("cannot be listed"));
        Ok(())
    }
}
