use assert_cmd::prelude::*;
use assert_fs::prelude::*;
use predicates::prelude::*;
use std::{
    fs::File,
    process::{Command, Stdio},
};

mod common;
use common::*;

mod snappy {
    use super::*;

    mod roundtrip {
        use super::*;

        /// Snappy roundtrip using explicit filenames
        /// Compressing: input = test.txt, output = test.txt.sz
        /// Extracting:  input = test.txt.sz, output = test.txt
        ///
        /// ``` bash
        /// cmprss snappy test.txt test.txt.sz
        /// cmprss snappy --extract --ignore-pipes test.txt.sz
        /// ```
        #[test]
        fn explicit() -> Result<(), Box<dyn std::error::Error>> {
            let file = create_test_file("test.txt", "garbage data for testing")?;
            let working_dir = create_working_dir()?;
            let archive = working_dir.child("test.txt.sz");
            archive.assert(predicate::path::missing());

            let mut compress = Command::cargo_bin("cmprss")?;
            compress
                .current_dir(&working_dir)
                .arg("snappy")
                .arg(file.path())
                .arg(archive.path());
            compress.assert().success();
            archive.assert(predicate::path::is_file());

            let mut extract = Command::cargo_bin("cmprss")?;
            extract
                .current_dir(&working_dir)
                .arg("snappy")
                .arg("--ignore-pipes")
                .arg("--extract")
                .arg(archive.path());
            extract.assert().success();

            // Assert the files are identical
            assert_files_equal(file.path(), &working_dir.child("test.txt"));

            Ok(())
        }

        /// Snappy roundtrip via the `sz` alias
        /// Compressing: input = test.txt, output = test.txt.sz
        /// Extracting:  input = test.txt.sz, output = out.txt
        ///
        /// ``` bash
        /// cmprss sz test.txt test.txt.sz
        /// cmprss sz --extract test.txt.sz out.txt
        /// ```
        #[test]
        fn alias() -> Result<(), Box<dyn std::error::Error>> {
            let file = create_test_file("test.txt", "garbage data for the alias test")?;
            let working_dir = create_working_dir()?;
            let archive = working_dir.child("test.txt.sz");

            let mut compress = Command::cargo_bin("cmprss")?;
            compress
                .current_dir(&working_dir)
                .arg("sz")
                .arg(file.path())
                .arg(archive.path());
            compress.assert().success();
            archive.assert(predicate::path::is_file());

            let output = working_dir.child("out.txt");
            let mut extract = Command::cargo_bin("cmprss")?;
            extract
                .current_dir(&working_dir)
                .arg("sz")
                .arg("--extract")
                .arg(archive.path())
                .arg(output.path());
            extract.assert().success();

            assert_files_equal(file.path(), output.path());

            Ok(())
        }

        /// Snappy roundtrip using stdin
        /// Compressing: input = stdin, output = test.txt.sz
        /// Extracting:  input = stdin(test.txt.sz), output = out.txt
        ///
        /// ``` bash
        /// cat test.txt | cmprss snappy test.txt.sz
        /// cat test.txt.sz | cmprss snappy --extract out.txt
        /// ```
        #[test]
        fn stdin() -> Result<(), Box<dyn std::error::Error>> {
            let file = create_test_file("test.txt", "garbage data for testing")?;
            let working_dir = create_working_dir()?;
            let archive = working_dir.child("test.txt.sz");
            archive.assert(predicate::path::missing());

            let mut compress = Command::cargo_bin("cmprss")?;
            compress
                .current_dir(&working_dir)
                .arg("snappy")
                .arg("test.txt.sz")
                .stdin(Stdio::from(File::open(file.path())?));
            compress.assert().success();
            archive.assert(predicate::path::is_file());

            let mut extract = Command::cargo_bin("cmprss")?;
            extract
                .current_dir(&working_dir)
                .arg("snappy")
                .stdin(Stdio::from(File::open(archive.path())?))
                .arg("--extract")
                .arg("out.txt");
            extract.assert().success();

            // Assert the files are identical
            assert_files_equal(file.path(), &working_dir.child("out.txt"));

            Ok(())
        }

        /// Snappy roundtrip using stdout
        /// Compressing: input = test.txt, output = stdout
        /// Extracting:  input = test.txt.sz, output = stdout
        ///
        /// ``` bash
        /// cmprss snappy test.txt > test.txt.sz
        /// cmprss snappy --extract test.txt.sz > out.txt
        /// ```
        #[test]
        fn stdout() -> Result<(), Box<dyn std::error::Error>> {
            let file = create_test_file("test.txt", "garbage data for testing")?;
            let working_dir = create_working_dir()?;
            let archive = working_dir.child("test.txt.sz");
            archive.assert(predicate::path::missing());

            let mut compress = Command::cargo_bin("cmprss")?;
            compress
                .current_dir(&working_dir)
                .arg("snappy")
                .arg(file.path())
                .stdout(Stdio::from(File::create(archive.path())?));
            compress.assert().success();
            archive.assert(predicate::path::is_file());

            let output = working_dir.child("out.txt");
            output.assert(predicate::path::missing());

            let mut extract = Command::cargo_bin("cmprss")?;
            extract
                .current_dir(&working_dir)
                .arg("snappy")
                .arg("--extract")
                .arg(archive.path())
                .stdout(Stdio::from(File::create(output.path())?));
            extract.assert().success();
            output.assert(predicate::path::is_file());

            assert_files_equal(file.path(), output.path());

            Ok(())
        }
    }
}
