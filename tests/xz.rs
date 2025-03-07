use assert_cmd::prelude::*;
use assert_fs::prelude::*;
use predicates::prelude::*;
use std::{
    fs::File,
    process::{Command, Stdio},
};

mod common;
use common::*;

mod xz {
    use super::*;

    mod roundtrip {
        use super::*;

        /// Xz roundtrip using files
        /// Compressing: input = test.txt, output = test.txt.xz
        /// Extracting:  input = test.txt.xz, output = test.txt
        ///
        /// ``` bash
        /// cmprss xz test.txt test.txt.xz
        /// cmprss xz --extract --ignore-pipes test.txt.xz
        /// ```
        #[test]
        fn explicit() -> Result<(), Box<dyn std::error::Error>> {
            let file = create_test_file("test.txt", "garbage data for testing")?;
            let working_dir = create_working_dir()?;
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
            assert_files_equal(file.path(), &working_dir.child("test.txt"));

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
        fn stdin() -> Result<(), Box<dyn std::error::Error>> {
            let file = create_test_file("test.txt", "garbage data for testing")?;
            let working_dir = create_working_dir()?;
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
            assert_files_equal(file.path(), &working_dir.child("out.txt"));

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
        fn stdout() -> Result<(), Box<dyn std::error::Error>> {
            let file = create_test_file("test.txt", "garbage data for testing")?;
            let working_dir = create_working_dir()?;
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
            extract.assert().success();

            // Assert the files are identical
            assert_files_equal(file.path(), &working_dir.child("out.txt"));

            Ok(())
        }
    }
}
