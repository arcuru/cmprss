use assert_cmd::prelude::*;
use assert_fs::prelude::*;
use predicates::prelude::*;
use std::{
    fs::File,
    process::{Command, Stdio},
};

mod common;
use common::*;

mod brotli {
    use super::*;

    mod roundtrip {
        use super::*;

        /// Brotli roundtrip using explicit filenames
        /// Compressing: input = test.txt, output = test.txt.br
        /// Extracting:  input = test.txt.br, output = test.txt
        ///
        /// ``` bash
        /// cmprss brotli test.txt test.txt.br
        /// cmprss brotli --extract --ignore-pipes test.txt.br
        /// ```
        #[test]
        fn explicit() -> Result<(), Box<dyn std::error::Error>> {
            let file = create_test_file("test.txt", "garbage data for testing")?;
            let working_dir = create_working_dir()?;
            let archive = working_dir.child("test.txt.br");
            archive.assert(predicate::path::missing());

            let mut compress = Command::cargo_bin("cmprss")?;
            compress
                .current_dir(&working_dir)
                .arg("brotli")
                .arg(file.path())
                .arg(archive.path());
            compress.assert().success();
            archive.assert(predicate::path::is_file());

            let mut extract = Command::cargo_bin("cmprss")?;
            extract
                .current_dir(&working_dir)
                .arg("brotli")
                .arg("--ignore-pipes")
                .arg("--extract")
                .arg(archive.path());
            extract.assert().success();

            // Assert the files are identical
            assert_files_equal(file.path(), &working_dir.child("test.txt"));

            Ok(())
        }

        /// Brotli roundtrip using the `br` alias
        /// Compressing: input = test.txt, output = test.txt.br
        /// Extracting:  input = test.txt.br, output = out.txt
        ///
        /// ``` bash
        /// cmprss br test.txt test.txt.br
        /// cmprss br --extract test.txt.br out.txt
        /// ```
        #[test]
        fn alias() -> Result<(), Box<dyn std::error::Error>> {
            let file = create_test_file("test.txt", "garbage data for the alias test")?;
            let working_dir = create_working_dir()?;
            let archive = working_dir.child("test.txt.br");

            let mut compress = Command::cargo_bin("cmprss")?;
            compress
                .current_dir(&working_dir)
                .arg("br")
                .arg(file.path())
                .arg(archive.path());
            compress.assert().success();
            archive.assert(predicate::path::is_file());

            let output = working_dir.child("out.txt");
            let mut extract = Command::cargo_bin("cmprss")?;
            extract
                .current_dir(&working_dir)
                .arg("br")
                .arg("--extract")
                .arg(archive.path())
                .arg(output.path());
            extract.assert().success();

            assert_files_equal(file.path(), output.path());

            Ok(())
        }

        /// Brotli roundtrip using stdin
        /// Compressing: input = stdin, output = test.txt.br
        /// Extracting:  input = stdin(test.txt.br), output = out.txt
        ///
        /// ``` bash
        /// cat test.txt | cmprss brotli test.txt.br
        /// cat test.txt.br | cmprss brotli --extract out.txt
        /// ```
        #[test]
        fn stdin() -> Result<(), Box<dyn std::error::Error>> {
            let file = create_test_file("test.txt", "garbage data for testing")?;
            let working_dir = create_working_dir()?;
            let archive = working_dir.child("test.txt.br");
            archive.assert(predicate::path::missing());

            // Pipe file to stdin
            let mut compress = Command::cargo_bin("cmprss")?;
            compress
                .current_dir(&working_dir)
                .arg("brotli")
                .arg("test.txt.br")
                .stdin(Stdio::from(File::open(file.path())?));
            compress.assert().success();
            archive.assert(predicate::path::is_file());

            let mut extract = Command::cargo_bin("cmprss")?;
            extract
                .current_dir(&working_dir)
                .arg("brotli")
                .stdin(Stdio::from(File::open(archive.path())?))
                .arg("--extract")
                .arg("out.txt");
            extract.assert().success();

            // Assert the files are identical
            assert_files_equal(file.path(), &working_dir.child("out.txt"));

            Ok(())
        }

        /// Brotli roundtrip using stdout
        /// Compressing: input = test.txt, output = stdout
        /// Extracting:  input = test.txt.br, output = stdout
        ///
        /// ``` bash
        /// cmprss brotli test.txt > test.txt.br
        /// cmprss brotli --extract test.txt.br > out.txt
        /// ```
        #[test]
        fn stdout() -> Result<(), Box<dyn std::error::Error>> {
            let file = create_test_file("test.txt", "garbage data for testing")?;
            let working_dir = create_working_dir()?;
            let archive = working_dir.child("test.txt.br");
            archive.assert(predicate::path::missing());

            // Compress file to stdout
            let mut compress = Command::cargo_bin("cmprss")?;
            compress
                .current_dir(&working_dir)
                .arg("brotli")
                .arg(file.path())
                .stdout(Stdio::from(File::create(archive.path())?));
            compress.assert().success();
            archive.assert(predicate::path::is_file());

            let output = working_dir.child("out.txt");
            output.assert(predicate::path::missing());

            let mut extract = Command::cargo_bin("cmprss")?;
            extract
                .current_dir(&working_dir)
                .arg("brotli")
                .arg("--extract")
                .arg(archive.path())
                .stdout(Stdio::from(File::create(output.path())?));
            extract.assert().success();
            output.assert(predicate::path::is_file());

            // Assert the files are identical
            assert_files_equal(file.path(), output.path());

            Ok(())
        }

        /// Brotli roundtrip with compression level
        /// Compressing: input = test.txt, output = test.txt.br, level = 11
        /// Extracting:  input = test.txt.br, output = test.txt
        ///
        /// ``` bash
        /// cmprss brotli --level 11 test.txt test.txt.br
        /// cmprss brotli --extract test.txt.br test.txt
        /// ```
        #[test]
        fn with_level() -> Result<(), Box<dyn std::error::Error>> {
            let file = create_test_file("test.txt", "garbage data for testing")?;
            let working_dir = create_working_dir()?;
            let archive = working_dir.child("test.txt.br");
            archive.assert(predicate::path::missing());

            let mut compress = Command::cargo_bin("cmprss")?;
            compress
                .current_dir(&working_dir)
                .arg("brotli")
                .arg("--level")
                .arg("11")
                .arg(file.path())
                .arg(archive.path());
            compress.assert().success();
            archive.assert(predicate::path::is_file());

            let output = working_dir.child("test.txt");

            let mut extract = Command::cargo_bin("cmprss")?;
            extract
                .current_dir(&working_dir)
                .arg("brotli")
                .arg("--extract")
                .arg(archive.path())
                .arg(output.path());
            extract.assert().success();

            // Assert the files are identical
            assert_files_equal(file.path(), output.path());

            Ok(())
        }
    }
}
