use assert_cmd::prelude::*;
use assert_fs::prelude::*;
use predicates::prelude::*;
use std::{path::PathBuf, process::Command};

mod common;
use common::*;

mod zip {
    use super::*;

    mod roundtrip {
        use super::*;

        /// Zip roundtrip with a single file
        ///
        /// ``` bash
        /// cmprss zip test.txt archive.zip
        /// cmprss zip --extract archive.zip .
        /// ```
        #[test]
        fn explicit() -> Result<(), Box<dyn std::error::Error>> {
            let file = create_test_file("test.txt", "garbage data for testing")?;
            let working_dir = create_working_dir()?;
            let archive = working_dir.child("archive.zip");
            archive.assert(predicate::path::missing());

            let mut compress = Command::cargo_bin("cmprss")?;
            compress.arg("zip").arg(file.path()).arg(archive.path());
            compress.assert().success();
            archive.assert(predicate::path::is_file());

            let mut extract = Command::cargo_bin("cmprss")?;
            extract
                .arg("zip")
                .arg("--extract")
                .arg(archive.path())
                .arg(working_dir.path());
            extract.assert().success();

            // Assert the files are identical
            assert_files_equal(file.path(), &working_dir.child("test.txt"));

            Ok(())
        }

        /// Zip roundtrip with multiple files
        ///
        /// ``` bash
        /// cmprss zip test.txt test2.txt archive.zip
        /// cmprss zip --extract archive.zip .
        /// ```
        #[test]
        fn explicit_two() -> Result<(), Box<dyn std::error::Error>> {
            let file = create_test_file("test.txt", "garbage data for testing")?;
            let file2 = create_test_file("test2.txt", "more garbage data for testing")?;
            let working_dir = create_working_dir()?;
            let archive = working_dir.child("archive.zip");
            archive.assert(predicate::path::missing());

            let mut compress = Command::cargo_bin("cmprss")?;
            compress
                .arg("zip")
                .arg(file.path())
                .arg(file2.path())
                .arg(archive.path());
            compress.assert().success();
            archive.assert(predicate::path::is_file());

            let mut extract = Command::cargo_bin("cmprss")?;
            extract
                .arg("zip")
                .arg("--extract")
                .arg(archive.path())
                .arg(working_dir.path());
            extract.assert().success();

            // Assert the files are identical
            assert_files_equal(file.path(), &working_dir.child("test.txt"));
            assert_files_equal(file2.path(), &working_dir.child("test2.txt"));

            Ok(())
        }

        /// Zip roundtrip with a directory
        ///
        /// ``` bash
        /// cmprss zip directory archive.zip
        /// cmprss zip --extract archive.zip output_dir
        /// ```
        #[test]
        fn directory() -> Result<(), Box<dyn std::error::Error>> {
            let dir = create_working_dir()?;
            let file = dir.child("test.txt");
            file.write_str("garbage data for testing")?;
            let file2 = dir.child("test2.txt");
            file2.write_str("more garbage data for testing")?;

            let working_dir = create_working_dir()?;
            let archive = working_dir.child("archive.zip");
            archive.assert(predicate::path::missing());

            let mut compress = Command::cargo_bin("cmprss")?;
            compress.arg("zip").arg(dir.path()).arg(archive.path());
            compress.assert().success();
            archive.assert(predicate::path::is_file());

            let extract_dir = working_dir.child("output");
            std::fs::create_dir_all(extract_dir.path())?;

            let mut extract = Command::cargo_bin("cmprss")?;
            extract
                .arg("zip")
                .arg("--extract")
                .arg(archive.path())
                .arg(extract_dir.path());
            extract.assert().success();

            // Assert the files are identical
            // Since the archive stores the entire directory, the extracted file is contained in the directory
            let dir_name: PathBuf = dir.path().file_name().unwrap().into();
            assert_files_equal(file.path(), &extract_dir.child(&dir_name).child("test.txt"));
            assert_files_equal(
                file2.path(),
                &extract_dir.child(&dir_name).child("test2.txt"),
            );

            Ok(())
        }
    }
}
