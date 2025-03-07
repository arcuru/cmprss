use assert_cmd::prelude::*;
use assert_fs::prelude::*;
use predicates::prelude::*;
use std::process::Command;

mod common;
use common::*;

mod tar {
    use super::*;

    mod roundtrip {
        use super::*;

        /// Tar roundtrip with a single file
        ///
        /// ``` bash
        /// cmprss tar test.txt archive.tar
        /// cmprss tar --extract archive.tar .
        /// ```
        #[test]
        fn explicit() -> Result<(), Box<dyn std::error::Error>> {
            let file = create_test_file("test.txt", "garbage data for testing")?;
            let working_dir = create_working_dir()?;
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
            assert_files_equal(file.path(), &working_dir.child("test.txt"));

            Ok(())
        }

        /// Tar roundtrip with multiple files
        ///
        /// ``` bash
        /// cmprss tar test.txt test2.txt archive.tar
        /// cmprss tar --extract archive.tar .
        /// ```
        #[test]
        fn explicit_two() -> Result<(), Box<dyn std::error::Error>> {
            let file = create_test_file("test.txt", "garbage data for testing")?;
            let file2 = create_test_file("test2.txt", "more garbage data for testing")?;
            let working_dir = create_working_dir()?;
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
            assert_files_equal(file.path(), &working_dir.child("test.txt"));
            assert_files_equal(file2.path(), &working_dir.child("test2.txt"));

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
        fn implicit() -> Result<(), Box<dyn std::error::Error>> {
            let file = create_test_file("test.txt", "garbage data for testing")?;
            let working_dir = create_persistent_working_dir()?;
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
            assert_files_equal(file.path(), &working_dir.child("test.txt"));

            Ok(())
        }

        /// Tar roundtrip with multiple files inferring output
        /// Uses the first file's name to generate the output filename
        /// Compressing: output = './test.txt.tar'
        /// Extracting:  output = '.'
        ///
        /// ``` bash
        /// cmprss tar test.txt test2.txt
        /// cmprss tar test.txt.tar
        /// ```
        #[test]
        fn implicit_two() -> Result<(), Box<dyn std::error::Error>> {
            let file = create_test_file("test.txt", "garbage data for testing")?;
            let file2 = create_test_file("test2.txt", "more garbage data for testing")?;
            let working_dir = create_persistent_working_dir()?;
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
                .arg("--ignore-pipes")
                .arg(archive.path());
            extract.assert().success();

            // Assert the files are identical
            assert_files_equal(file.path(), &working_dir.child("test.txt"));
            assert_files_equal(file2.path(), &working_dir.child("test2.txt"));

            Ok(())
        }
    }
}
