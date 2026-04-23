use assert_cmd::prelude::*;
use assert_fs::prelude::*;
use predicates::prelude::*;
use std::{path::PathBuf, process::Command};

mod common;
use common::*;

mod sevenz {
    use super::*;

    mod roundtrip {
        use super::*;

        /// 7z roundtrip with a single file
        ///
        /// ``` bash
        /// cmprss 7z test.txt archive.7z
        /// cmprss 7z --extract archive.7z .
        /// ```
        #[test]
        fn explicit() -> Result<(), Box<dyn std::error::Error>> {
            let file = create_test_file("test.txt", "garbage data for testing")?;
            let working_dir = create_working_dir()?;
            let archive = working_dir.child("archive.7z");
            archive.assert(predicate::path::missing());

            let mut compress = Command::cargo_bin("cmprss")?;
            compress.arg("7z").arg(file.path()).arg(archive.path());
            compress.assert().success();
            archive.assert(predicate::path::is_file());

            let mut extract = Command::cargo_bin("cmprss")?;
            extract
                .arg("7z")
                .arg("--extract")
                .arg(archive.path())
                .arg(working_dir.path());
            extract.assert().success();

            assert_files_equal(file.path(), &working_dir.child("test.txt"));

            Ok(())
        }

        /// 7z roundtrip with multiple files
        ///
        /// ``` bash
        /// cmprss 7z test.txt test2.txt archive.7z
        /// cmprss 7z --extract archive.7z .
        /// ```
        #[test]
        fn explicit_two() -> Result<(), Box<dyn std::error::Error>> {
            let file = create_test_file("test.txt", "garbage data for testing")?;
            let file2 = create_test_file("test2.txt", "more garbage data for testing")?;
            let working_dir = create_working_dir()?;
            let archive = working_dir.child("archive.7z");
            archive.assert(predicate::path::missing());

            let mut compress = Command::cargo_bin("cmprss")?;
            compress
                .arg("7z")
                .arg(file.path())
                .arg(file2.path())
                .arg(archive.path());
            compress.assert().success();
            archive.assert(predicate::path::is_file());

            let mut extract = Command::cargo_bin("cmprss")?;
            extract
                .arg("7z")
                .arg("--extract")
                .arg(archive.path())
                .arg(working_dir.path());
            extract.assert().success();

            assert_files_equal(file.path(), &working_dir.child("test.txt"));
            assert_files_equal(file2.path(), &working_dir.child("test2.txt"));

            Ok(())
        }

        /// 7z roundtrip with a directory
        ///
        /// ``` bash
        /// cmprss 7z directory archive.7z
        /// cmprss 7z --extract archive.7z output_dir
        /// ```
        #[test]
        fn directory() -> Result<(), Box<dyn std::error::Error>> {
            let dir = create_working_dir()?;
            let file = dir.child("test.txt");
            file.write_str("garbage data for testing")?;
            let file2 = dir.child("test2.txt");
            file2.write_str("more garbage data for testing")?;

            let working_dir = create_working_dir()?;
            let archive = working_dir.child("archive.7z");
            archive.assert(predicate::path::missing());

            let mut compress = Command::cargo_bin("cmprss")?;
            compress.arg("7z").arg(dir.path()).arg(archive.path());
            compress.assert().success();
            archive.assert(predicate::path::is_file());

            let extract_dir = working_dir.child("output");
            std::fs::create_dir_all(extract_dir.path())?;

            let mut extract = Command::cargo_bin("cmprss")?;
            extract
                .arg("7z")
                .arg("--extract")
                .arg(archive.path())
                .arg(extract_dir.path());
            extract.assert().success();

            let dir_name: PathBuf = dir.path().file_name().unwrap().into();
            assert_files_equal(file.path(), &extract_dir.child(&dir_name).child("test.txt"));
            assert_files_equal(
                file2.path(),
                &extract_dir.child(&dir_name).child("test2.txt"),
            );

            Ok(())
        }

        /// 7z listing the contents of an archive
        ///
        /// ``` bash
        /// cmprss 7z --list archive.7z
        /// ```
        #[test]
        fn list() -> Result<(), Box<dyn std::error::Error>> {
            let file = create_test_file("listed.txt", "entry data")?;
            let working_dir = create_working_dir()?;
            let archive = working_dir.child("archive.7z");

            let mut compress = Command::cargo_bin("cmprss")?;
            compress.arg("7z").arg(file.path()).arg(archive.path());
            compress.assert().success();

            let mut list = Command::cargo_bin("cmprss")?;
            list.arg("7z").arg("--list").arg(archive.path());
            list.assert()
                .success()
                .stdout(predicate::str::contains("listed.txt"));

            Ok(())
        }

        /// 7z via the `sevenz` alias
        ///
        /// ``` bash
        /// cmprss sevenz test.txt archive.7z
        /// ```
        #[test]
        fn alias() -> Result<(), Box<dyn std::error::Error>> {
            let file = create_test_file("test.txt", "garbage data for the alias test")?;
            let working_dir = create_working_dir()?;
            let archive = working_dir.child("archive.7z");

            let mut compress = Command::cargo_bin("cmprss")?;
            compress.arg("sevenz").arg(file.path()).arg(archive.path());
            compress.assert().success();
            archive.assert(predicate::path::is_file());

            Ok(())
        }
    }
}
