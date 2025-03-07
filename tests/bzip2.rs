use assert_cmd::prelude::*;
use assert_fs::prelude::*;
use predicates::prelude::*;
use std::{
    fs::File,
    process::{Command, Stdio},
};

mod common;
use common::*;

mod bzip2 {
    use super::*;

    mod roundtrip {
        use super::*;

        /// Bzip2 roundtrip using files
        /// Compressing: input = test.txt, output = test.txt.bz2
        /// Extracting:  input = test.txt.bz2, output = test.txt
        ///
        /// ``` bash
        /// cmprss bzip2 test.txt test.txt.bz2
        /// cmprss bzip2 --extract --ignore-pipes test.txt.bz2
        /// ```
        #[test]
        fn explicit() -> Result<(), Box<dyn std::error::Error>> {
            let file = create_test_file("test.txt", "garbage data for testing")?;
            let working_dir = create_working_dir()?;
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
            assert_files_equal(file.path(), &working_dir.child("test.txt"));

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
        fn stdin() -> Result<(), Box<dyn std::error::Error>> {
            let file = create_test_file("test.txt", "garbage data for testing")?;
            let working_dir = create_working_dir()?;
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
            assert_files_equal(file.path(), &working_dir.child("out.txt"));

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
        fn stdout() -> Result<(), Box<dyn std::error::Error>> {
            let file = create_test_file("test.txt", "garbage data for testing")?;
            let working_dir = create_working_dir()?;
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
            extract.assert().success();

            // Assert the files are identical
            assert_files_equal(file.path(), &working_dir.child("out.txt"));

            Ok(())
        }
    }
}
