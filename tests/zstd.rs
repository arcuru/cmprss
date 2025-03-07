use assert_cmd::prelude::*;
use assert_fs::prelude::*;
use predicates::prelude::*;
use std::{
    fs::File,
    process::{Command, Stdio},
};

mod common;
use common::*;

mod zstd {
    use super::*;

    mod roundtrip {
        use super::*;

        /// Zstd roundtrip using explicit filenames
        /// Compressing: input = test.txt, output = test.txt.zst
        /// Extracting:  input = test.txt.zst, output = test.txt
        ///
        /// ``` bash
        /// cmprss zstd test.txt test.txt.zst
        /// cmprss zstd --extract --ignore-pipes test.txt.zst
        /// ```
        #[test]
        fn explicit() -> Result<(), Box<dyn std::error::Error>> {
            let file = create_test_file("test.txt", "garbage data for testing")?;
            let working_dir = create_working_dir()?;
            let archive = working_dir.child("test.txt.zst");
            archive.assert(predicate::path::missing());

            let mut compress = Command::cargo_bin("cmprss")?;
            compress
                .current_dir(&working_dir)
                .arg("zstd")
                .arg(file.path())
                .arg(archive.path());
            compress.assert().success();
            archive.assert(predicate::path::is_file());

            let mut extract = Command::cargo_bin("cmprss")?;
            extract
                .current_dir(&working_dir)
                .arg("zstd")
                .arg("--ignore-pipes")
                .arg("--extract")
                .arg(archive.path());
            extract.assert().success();

            // Assert the files are identical
            assert_files_equal(file.path(), &working_dir.child("test.txt"));

            Ok(())
        }

        /// Zstd roundtrip using stdin
        /// Compressing: input = stdin, output = test.txt.zst
        /// Extracting:  input = stdin(test.txt.zst), output = test.txt
        ///
        /// ``` bash
        /// cat test.txt | cmprss zstd test.txt.zst
        /// cat test.txt.zst | cmprss zstd --extract out.txt
        /// ```
        #[test]
        fn stdin() -> Result<(), Box<dyn std::error::Error>> {
            let file = create_test_file("test.txt", "garbage data for testing")?;
            let working_dir = create_working_dir()?;
            let archive = working_dir.child("test.txt.zst");
            archive.assert(predicate::path::missing());

            // Pipe file to stdin
            let mut compress = Command::cargo_bin("cmprss")?;
            compress
                .current_dir(&working_dir)
                .arg("zstd")
                .arg("test.txt.zst")
                .stdin(Stdio::from(File::open(file.path())?));
            compress.assert().success();
            archive.assert(predicate::path::is_file());

            let mut extract = Command::cargo_bin("cmprss")?;
            extract
                .current_dir(&working_dir)
                .arg("zstd")
                .stdin(Stdio::from(File::open(archive.path())?))
                .arg("--extract")
                .arg("out.txt");
            extract.assert().success();

            // Assert the files are identical
            assert_files_equal(file.path(), &working_dir.child("out.txt"));

            Ok(())
        }

        /// Zstd roundtrip using stdout
        /// Compressing: input = test.txt, output = stdout
        /// Extracting:  input = test.txt.zst, output = stdout
        ///
        /// ``` bash
        /// cmprss zstd test.txt > test.txt.zst
        /// cmprss zstd --extract test.txt.zst > out.txt
        /// ```
        #[test]
        fn stdout() -> Result<(), Box<dyn std::error::Error>> {
            let file = create_test_file("test.txt", "garbage data for testing")?;
            let working_dir = create_working_dir()?;
            let archive = working_dir.child("test.txt.zst");
            archive.assert(predicate::path::missing());

            // Compress file to stdout
            let mut compress = Command::cargo_bin("cmprss")?;
            compress
                .current_dir(&working_dir)
                .arg("zstd")
                .arg(file.path())
                .stdout(Stdio::from(File::create(archive.path())?));
            compress.assert().success();
            archive.assert(predicate::path::is_file());

            let output = working_dir.child("out.txt");
            output.assert(predicate::path::missing());

            let mut extract = Command::cargo_bin("cmprss")?;
            extract
                .current_dir(&working_dir)
                .arg("zstd")
                .arg("--extract")
                .arg(archive.path())
                .stdout(Stdio::from(File::create(output.path())?));
            extract.assert().success();
            output.assert(predicate::path::is_file());

            // Assert the files are identical
            assert_files_equal(file.path(), output.path());

            Ok(())
        }

        /// Zstd roundtrip with compression level
        /// Compressing: input = test.txt, output = test.txt.zst, level = 9
        /// Extracting:  input = test.txt.zst, output = test.txt
        ///
        /// ``` bash
        /// cmprss zstd --level 9 test.txt test.txt.zst
        /// cmprss zstd --extract test.txt.zst test.txt
        /// ```
        #[test]
        fn with_level() -> Result<(), Box<dyn std::error::Error>> {
            let file = create_test_file("test.txt", "garbage data for testing")?;
            let working_dir = create_working_dir()?;
            let archive = working_dir.child("test.txt.zst");
            archive.assert(predicate::path::missing());

            let mut compress = Command::cargo_bin("cmprss")?;
            compress
                .current_dir(&working_dir)
                .arg("zstd")
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
                .arg("zstd")
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
