use assert_cmd::prelude::*;
use assert_fs::prelude::*;
use predicates::prelude::*;
use std::{
    fs::File,
    process::{Command, Stdio},
};

mod common;
use common::*;

mod lz4 {
    use super::*;

    mod roundtrip {
        use super::*;

        /// LZ4 roundtrip using explicit filenames
        /// Compressing: input = test.txt, output = test.txt.lz4
        /// Extracting:  input = test.txt.lz4, output = test.txt
        ///
        /// ``` bash
        /// cmprss lz4 test.txt test.txt.lz4
        /// cmprss lz4 --extract test.txt.lz4 test.txt
        /// ```
        #[test]
        fn explicit() -> Result<(), Box<dyn std::error::Error>> {
            let file = create_test_file("test.txt", "This is a test file for LZ4 compression.")?;
            let working_dir = create_working_dir()?;
            let archive = working_dir.child("test.txt.lz4");
            let extracted_file = working_dir.child("test_extracted.txt");

            // Compress the file
            let mut compress = Command::cargo_bin("cmprss")?;
            compress
                .arg("lz4")
                .arg("--compress")
                .arg(file.path())
                .arg(archive.path())
                .arg("--progress=off");
            compress.assert().success();
            archive.assert(predicate::path::exists());

            // Extract the file
            let mut extract = Command::cargo_bin("cmprss")?;
            extract
                .arg("lz4")
                .arg("--extract")
                .arg(archive.path())
                .arg(extracted_file.path())
                .arg("--progress=off");
            extract.assert().success();
            extracted_file.assert(predicate::path::exists());

            // Verify the contents
            assert_files_equal(file.path(), extracted_file.path());

            Ok(())
        }

        /// LZ4 roundtrip using stdin
        /// Compressing: input = stdin, output = test.txt.lz4
        /// Extracting:  input = test.txt.lz4, output = test.txt
        ///
        /// ``` bash
        /// cat test.txt | cmprss lz4 test.txt.lz4
        /// cmprss lz4 --extract test.txt.lz4 test.txt
        /// ```
        #[test]
        fn stdin() -> Result<(), Box<dyn std::error::Error>> {
            let file = create_test_file(
                "test.txt",
                "This is a test file for LZ4 compression via stdin.",
            )?;
            let working_dir = create_working_dir()?;
            let archive = working_dir.child("test.txt.lz4");
            let extracted_file = working_dir.child("test_extracted.txt");

            // Compress the file via stdin
            let mut compress = Command::cargo_bin("cmprss")?;
            compress
                .arg("lz4")
                .arg("--compress")
                .arg("--output")
                .arg(archive.path())
                .arg("--progress=off")
                .stdin(Stdio::from(File::open(file.path())?));
            compress.assert().success();
            archive.assert(predicate::path::exists());

            // Extract the file
            let mut extract = Command::cargo_bin("cmprss")?;
            extract
                .arg("lz4")
                .arg("--extract")
                .arg(archive.path())
                .arg(extracted_file.path())
                .arg("--progress=off");
            extract.assert().success();
            extracted_file.assert(predicate::path::exists());

            // Verify the contents
            assert_files_equal(file.path(), extracted_file.path());

            Ok(())
        }

        /// LZ4 roundtrip using stdout
        /// Compressing: input = test.txt, output = stdout
        /// Extracting:  input = test.txt.lz4, output = stdout
        ///
        /// ``` bash
        /// cmprss lz4 test.txt > test.txt.lz4
        /// cmprss lz4 --extract test.txt.lz4 > test.txt
        /// ```
        #[test]
        fn stdout() -> Result<(), Box<dyn std::error::Error>> {
            let file = create_test_file(
                "test.txt",
                "This is a test file for LZ4 compression to stdout.",
            )?;
            let working_dir = create_working_dir()?;
            let archive = working_dir.child("test.txt.lz4");
            let extracted_file = working_dir.child("test_extracted.txt");

            // Compress the file
            let mut compress = Command::cargo_bin("cmprss")?;
            compress
                .arg("lz4")
                .arg("--compress")
                .arg(file.path())
                .arg("--progress=off")
                .stdout(Stdio::from(File::create(archive.path())?));
            compress.assert().success();
            archive.assert(predicate::path::exists());

            // Extract the file to stdout
            let mut extract = Command::cargo_bin("cmprss")?;
            extract
                .arg("lz4")
                .arg("--extract")
                .arg(archive.path())
                .arg("--progress=off")
                .stdout(Stdio::from(File::create(extracted_file.path())?));
            extract.assert().success();
            extracted_file.assert(predicate::path::exists());

            // Verify the contents
            assert_files_equal(file.path(), extracted_file.path());

            Ok(())
        }
    }
}
