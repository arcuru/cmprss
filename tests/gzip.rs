use assert_cmd::prelude::*;
use assert_fs::prelude::*;
use predicates::prelude::*;
use std::{
    fs::File,
    process::{Command, Stdio},
};

mod common;
use common::*;

mod gzip {
    use super::*;

    mod roundtrip {
        use super::*;

        /// Gzip roundtrip using stdin
        /// Compressing: input = stdin, output = test.txt.gz
        /// Extracting:  input = test.txt.gz, output = test.txt
        ///
        /// ``` bash
        /// cat test.txt | cmprss gzip test.txt.gz
        /// cmprss gzip --ignore-pipes --extract test.txt.gz
        /// ```
        #[test]
        fn stdin() -> Result<(), Box<dyn std::error::Error>> {
            let file = create_test_file("test.txt", "garbage data for testing")?;
            let working_dir = create_working_dir()?;
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
            assert_files_equal(file.path(), &working_dir.child("test.txt"));

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
        fn inferred_output_filenames() -> Result<(), Box<dyn std::error::Error>> {
            let file = create_test_file("test.txt", "garbage data for testing")?;
            let working_dir = create_working_dir()?;
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
            assert_files_equal(file.path(), &working_dir.child("archive"));

            Ok(())
        }
    }
}
