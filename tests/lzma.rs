use assert_cmd::prelude::*;
use assert_fs::prelude::*;
use predicates::prelude::*;
use std::{
    fs::File,
    process::{Command, Stdio},
};

mod common;
use common::*;

mod lzma {
    use super::*;

    mod roundtrip {
        use super::*;

        /// Lzma roundtrip using explicit filenames
        /// Compressing: input = test.txt, output = test.txt.lzma
        /// Extracting:  input = test.txt.lzma, output = test.txt
        ///
        /// ``` bash
        /// cmprss lzma test.txt test.txt.lzma
        /// cmprss lzma --extract --ignore-pipes test.txt.lzma
        /// ```
        #[test]
        fn explicit() -> Result<(), Box<dyn std::error::Error>> {
            let file = create_test_file("test.txt", "garbage data for testing")?;
            let working_dir = create_working_dir()?;
            let archive = working_dir.child("test.txt.lzma");
            archive.assert(predicate::path::missing());

            let mut compress = Command::cargo_bin("cmprss")?;
            compress
                .current_dir(&working_dir)
                .arg("lzma")
                .arg(file.path())
                .arg(archive.path());
            compress.assert().success();
            archive.assert(predicate::path::is_file());

            let mut extract = Command::cargo_bin("cmprss")?;
            extract
                .current_dir(&working_dir)
                .arg("lzma")
                .arg("--ignore-pipes")
                .arg("--extract")
                .arg(archive.path());
            extract.assert().success();

            // Assert the files are identical
            assert_files_equal(file.path(), &working_dir.child("test.txt"));

            Ok(())
        }

        /// Lzma roundtrip using stdin
        /// Compressing: input = stdin, output = test.txt.lzma
        /// Extracting:  input = stdin(test.txt.lzma), output = out.txt
        ///
        /// ``` bash
        /// cat test.txt | cmprss lzma test.txt.lzma
        /// cat test.txt.lzma | cmprss lzma --extract out.txt
        /// ```
        #[test]
        fn stdin() -> Result<(), Box<dyn std::error::Error>> {
            let file = create_test_file("test.txt", "garbage data for testing")?;
            let working_dir = create_working_dir()?;
            let archive = working_dir.child("test.txt.lzma");
            archive.assert(predicate::path::missing());

            let mut compress = Command::cargo_bin("cmprss")?;
            compress
                .current_dir(&working_dir)
                .arg("lzma")
                .arg("test.txt.lzma")
                .stdin(Stdio::from(File::open(file.path())?));
            compress.assert().success();
            archive.assert(predicate::path::is_file());

            let mut extract = Command::cargo_bin("cmprss")?;
            extract
                .current_dir(&working_dir)
                .arg("lzma")
                .stdin(Stdio::from(File::open(archive.path())?))
                .arg("--extract")
                .arg("out.txt");
            extract.assert().success();

            // Assert the files are identical
            assert_files_equal(file.path(), &working_dir.child("out.txt"));

            Ok(())
        }

        /// Lzma roundtrip using stdout
        /// Compressing: input = test.txt, output = stdout
        /// Extracting:  input = test.txt.lzma, output = stdout
        ///
        /// ``` bash
        /// cmprss lzma test.txt > test.txt.lzma
        /// cmprss lzma --extract test.txt.lzma > out.txt
        /// ```
        #[test]
        fn stdout() -> Result<(), Box<dyn std::error::Error>> {
            let file = create_test_file("test.txt", "garbage data for testing")?;
            let working_dir = create_working_dir()?;
            let archive = working_dir.child("test.txt.lzma");
            archive.assert(predicate::path::missing());

            let mut compress = Command::cargo_bin("cmprss")?;
            compress
                .current_dir(&working_dir)
                .arg("lzma")
                .arg(file.path())
                .stdout(Stdio::from(File::create(archive.path())?));
            compress.assert().success();
            archive.assert(predicate::path::is_file());

            let output = working_dir.child("out.txt");
            output.assert(predicate::path::missing());

            let mut extract = Command::cargo_bin("cmprss")?;
            extract
                .current_dir(&working_dir)
                .arg("lzma")
                .arg("--extract")
                .arg(archive.path())
                .stdout(Stdio::from(File::create(output.path())?));
            extract.assert().success();
            output.assert(predicate::path::is_file());

            assert_files_equal(file.path(), output.path());

            Ok(())
        }

        /// Lzma roundtrip with compression level
        /// Compressing: input = test.txt, output = test.txt.lzma, level = 9
        /// Extracting:  input = test.txt.lzma, output = test.txt
        ///
        /// ``` bash
        /// cmprss lzma --level 9 test.txt test.txt.lzma
        /// cmprss lzma --extract test.txt.lzma test.txt
        /// ```
        #[test]
        fn with_level() -> Result<(), Box<dyn std::error::Error>> {
            let file = create_test_file("test.txt", "garbage data for testing")?;
            let working_dir = create_working_dir()?;
            let archive = working_dir.child("test.txt.lzma");
            archive.assert(predicate::path::missing());

            let mut compress = Command::cargo_bin("cmprss")?;
            compress
                .current_dir(&working_dir)
                .arg("lzma")
                .arg("--level")
                .arg("9")
                .arg(file.path())
                .arg(archive.path());
            compress.assert().success();
            archive.assert(predicate::path::is_file());

            let output = working_dir.child("test.txt");

            let mut extract = Command::cargo_bin("cmprss")?;
            extract
                .current_dir(&working_dir)
                .arg("lzma")
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
