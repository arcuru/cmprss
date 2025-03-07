use assert_cmd::prelude::*;
use assert_fs::prelude::*;
use predicates::prelude::*;
use std::{
    fs::File,
    process::{Command, Stdio},
};

mod common;
use common::*;

mod magic {
    use super::*;

    mod roundtrip {
        use super::*;

        /// Magic roundtrip using stdin
        /// Compressing: input = stdin, output = test.txt.gz
        /// Extracting:  input = test.txt.gz, output = test.txt
        ///
        /// ``` bash
        /// cat test.txt | cmprss test.txt.gz
        /// cmprss gz --extract test.txt.gz out.txt
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
                .arg("--ignore-stdout")
                .arg("test.txt.gz")
                .stdin(Stdio::from(File::open(file.path())?));
            compress.assert().success();
            archive.assert(predicate::path::is_file());

            let mut extract = Command::cargo_bin("cmprss")?;
            extract
                .current_dir(&working_dir)
                .arg("gz")
                .arg("--ignore-pipes")
                .arg("--extract")
                .arg("test.txt.gz");
            extract.assert().success();

            // Assert the files are identical
            assert_files_equal(file.path(), &working_dir.child("test.txt"));

            Ok(())
        }

        /// Magic roundtrip using files
        /// Compressing: input = test.txt, output = test.txt.gz
        /// Extracting:  input = test.txt.gz, output = stdout
        ///
        /// ``` bash
        /// cmprss test.txt test.txt.gz
        /// cmprss test.txt.gz out.txt
        /// ```
        #[test]
        fn files() -> Result<(), Box<dyn std::error::Error>> {
            let file = create_test_file("test.txt", "garbage data for testing")?;
            let working_dir = create_working_dir()?;
            let archive = working_dir.child("test.txt.gz");
            archive.assert(predicate::path::missing());

            // Compress file to an archive
            let mut compress = Command::cargo_bin("cmprss")?;
            compress
                .current_dir(&working_dir)
                .arg("--ignore-pipes")
                .arg(file.path())
                .arg("test.txt.gz");
            compress.assert().success();
            archive.assert(predicate::path::is_file());

            // Extract file to given file
            let mut extract = Command::cargo_bin("cmprss")?;
            extract
                .current_dir(&working_dir)
                .arg("--ignore-pipes")
                .arg("test.txt.gz")
                .arg("out.txt");
            extract.assert().success();

            // Assert the files are identical
            assert_files_equal(file.path(), &working_dir.child("out.txt"));

            Ok(())
        }

        /// Magic roundtrip using stdout decompression
        /// Compressing: input = test.txt, output = test.txt.gz
        /// Extracting:  input = test.txt.gz, output = stdout
        ///
        /// ``` bash
        /// cmprss test.txt test.txt.gz
        /// cmprss test.txt.gz > out.txt
        /// ```
        #[test]
        fn stdout_decompression() -> Result<(), Box<dyn std::error::Error>> {
            let file = create_test_file("test.txt", "garbage data for testing")?;
            let working_dir = create_working_dir()?;
            let archive = working_dir.child("test.txt.gz");
            archive.assert(predicate::path::missing());

            let out_file = working_dir.child("out.txt");

            // Compress file to an archive
            let mut compress = Command::cargo_bin("cmprss")?;
            compress
                .current_dir(&working_dir)
                .arg("--ignore-pipes")
                .arg(file.path())
                .arg("test.txt.gz");
            compress.assert().success();
            archive.assert(predicate::path::is_file());

            // Extract file to stdout
            let mut extract = Command::cargo_bin("cmprss")?;
            extract
                .current_dir(&working_dir)
                .arg("--ignore-stdin")
                .arg("test.txt.gz")
                .stdout(Stdio::from(File::create(&out_file)?));
            extract.assert().success();

            // Assert the files are identical
            assert_files_equal(file.path(), out_file.path());

            Ok(())
        }

        /// Magic roundtrip using stdin compression
        /// Compressing: input = stdin, output = test.txt.gz
        /// Extracting:  input = test.txt.gz, output = test.txt
        ///
        /// ``` bash
        /// cat test.txt | cmprss test.txt.gz
        /// cmprss test.txt.gz out.txt
        /// ```
        #[test]
        fn stdin_compression() -> Result<(), Box<dyn std::error::Error>> {
            let file = create_test_file("test.txt", "garbage data for testing")?;
            let working_dir = create_working_dir()?;
            let archive = working_dir.child("test.txt.gz");
            archive.assert(predicate::path::missing());

            let out_file = working_dir.child("out.txt");

            // Compress stdin to an archive
            let mut compress = Command::cargo_bin("cmprss")?;
            compress
                .current_dir(&working_dir)
                .arg("--ignore-stdout")
                .arg("test.txt.gz")
                .stdin(Stdio::from(File::open(file.path())?));
            compress.assert().success();
            archive.assert(predicate::path::is_file());

            // Extract file to given file
            let mut extract = Command::cargo_bin("cmprss")?;
            extract
                .current_dir(&working_dir)
                .arg("--ignore-pipes")
                .arg("test.txt.gz")
                .arg(out_file.path());
            extract.assert().success();

            // Assert the files are identical
            assert_files_equal(file.path(), out_file.path());

            Ok(())
        }

        /// Magic roundtrip using default filenames
        /// Compressing: input = test.txt, output = test.txt.gz
        /// Extracting:  input = test.txt.gz, output = <default>
        ///
        /// ``` bash
        /// cmprss test.txt test.txt.gz
        /// cmprss test.txt.gz
        /// ```
        #[test]
        fn default_filenames() -> Result<(), Box<dyn std::error::Error>> {
            let file = create_test_file("test.txt", "garbage data for testing")?;
            let working_dir = create_working_dir()?;
            let archive = working_dir.child("test.txt.gz");
            archive.assert(predicate::path::missing());

            // Compress file to an archive
            let mut compress = Command::cargo_bin("cmprss")?;
            compress
                .current_dir(&working_dir)
                .arg("--ignore-pipes")
                .arg(file.path())
                .arg("test.txt.gz");
            compress.assert().success();
            archive.assert(predicate::path::is_file());

            // Extract file to default filename
            let mut extract = Command::cargo_bin("cmprss")?;
            extract
                .current_dir(&working_dir)
                .arg("--ignore-pipes")
                .arg("test.txt.gz");
            extract.assert().success();

            // Assert the files are identical
            assert_files_equal(file.path(), &working_dir.child("test.txt"));

            Ok(())
        }

        /// Magic roundtrip using multiple files with tar
        /// Compressing: input = test.txt/test2.txt, output = archive.tar
        /// Extracting:  input = archive.tar, output = <default>
        ///
        /// ``` bash
        /// cmprss test.txt test2.txt archive.tar
        /// cmprss archive.tar
        /// ```
        #[test]
        fn multiple_files_tar() -> Result<(), Box<dyn std::error::Error>> {
            let file = create_test_file("test.txt", "garbage data for testing")?;
            let file2 = create_test_file("test2.txt", "more garbage data for testing")?;
            let working_dir = create_working_dir()?;
            let archive = working_dir.child("archive.tar");
            archive.assert(predicate::path::missing());

            // Compress files to an archive
            let mut compress = Command::cargo_bin("cmprss")?;
            compress
                .current_dir(&working_dir)
                .arg("--ignore-pipes")
                .arg(file.path())
                .arg(file2.path())
                .arg("archive.tar");
            compress.assert().success();
            archive.assert(predicate::path::is_file());

            // Extract file to default filename
            let mut extract = Command::cargo_bin("cmprss")?;
            extract
                .current_dir(&working_dir)
                .arg("--ignore-pipes")
                .arg("archive.tar");
            extract.assert().success();

            // Assert the files are identical
            assert_files_equal(file.path(), &working_dir.child("test.txt"));
            assert_files_equal(file2.path(), &working_dir.child("test2.txt"));

            Ok(())
        }

        /// Magic roundtrip with tar.gz
        /// Infer things as much as possible
        /// Compressing: input = test.txt + test2.txt, output = test.tar.gz
        /// Extracting:  input = test.tar.gz, output = test.txt + test2.txt
        ///
        /// ``` bash
        /// cmprss test.txt test2.txt archive.tar
        /// cmprss archive.tar archive.tar.gz
        /// cmprss archive.tar.gz archive.tar
        /// cmprss archive.tar
        /// ```
        #[test]
        fn tar_gz() -> Result<(), Box<dyn std::error::Error>> {
            let file = create_test_file("test.txt", "garbage data for testing")?;
            let file2 = create_test_file("test2.txt", "more garbage data for testing")?;
            let working_dir = create_working_dir()?;
            let archive = working_dir.child("archive.tar");
            archive.assert(predicate::path::missing());
            let archive2 = working_dir.child("archive.tar.gz");
            archive2.assert(predicate::path::missing());

            let extract_dir = create_working_dir()?;

            // Compress files to an archive
            let mut compress = Command::cargo_bin("cmprss")?;
            compress
                .current_dir(&working_dir)
                .arg("--ignore-pipes")
                .arg(file.path())
                .arg(file2.path())
                .arg("archive.tar");
            compress.assert().success();
            archive.assert(predicate::path::is_file());

            // Compress tar to an archive
            let mut compress2 = Command::cargo_bin("cmprss")?;
            compress2
                .current_dir(&working_dir)
                .arg("--ignore-pipes")
                .arg("archive.tar")
                .arg("archive.tar.gz");
            compress2.assert().success();
            archive2.assert(predicate::path::is_file());

            // Extract file to default filename
            let mut extract = Command::cargo_bin("cmprss")?;
            extract
                .current_dir(&extract_dir)
                .arg("--ignore-pipes")
                .arg(archive2.path())
                .arg("archive.tar");
            extract.assert().success();

            // Extract file to default filename
            let mut extract2 = Command::cargo_bin("cmprss")?;
            extract2
                .current_dir(&extract_dir)
                .arg("--ignore-pipes")
                .arg("archive.tar");
            extract2.assert().success();

            // Assert the files are identical
            assert_files_equal(file.path(), &extract_dir.child("test.txt"));
            assert_files_equal(file2.path(), &extract_dir.child("test2.txt"));

            Ok(())
        }

        /// Magic roundtrip with tar.gz using pipes
        /// Infer things as much as possible
        /// Compressing: input = test.txt + test2.txt, output = test.tar.gz
        /// Extracting:  input = test.tar.gz, output = test.txt + test2.txt
        ///
        /// ``` bash
        /// cmprss tar test.txt test2.txt | cmprss gzip | cmprss gzip --extract | cmprss tar --extract
        /// ```
        #[test]
        fn tar_gz_pipes() -> Result<(), Box<dyn std::error::Error>> {
            let file = create_test_file("test.txt", "garbage data for testing")?;
            let file2 = create_test_file("test2.txt", "more garbage data for testing")?;
            let working_dir = create_working_dir()?;
            let tee1 = working_dir.child("tee1");
            let tee2 = working_dir.child("tee2");
            let tee3 = working_dir.child("tee3");

            let extract_dir = create_working_dir()?;

            let mut compress = Command::cargo_bin("cmprss")?;
            compress
                .current_dir(&working_dir)
                .arg("tar")
                .arg("--ignore-stdin")
                .arg(file.path())
                .arg(file2.path())
                .stdout(Stdio::from(File::create(tee1.path())?));
            compress.assert().success();
            tee1.assert(predicate::path::is_file());

            let mut compress2 = Command::cargo_bin("cmprss")?;
            compress2
                .current_dir(&working_dir)
                .arg("gzip")
                .stdin(Stdio::from(File::open(tee1.path())?))
                .stdout(Stdio::from(File::create(tee2.path())?));
            compress2.assert().success();
            tee2.assert(predicate::path::is_file());

            let mut extract = Command::cargo_bin("cmprss")?;
            extract
                .current_dir(&working_dir)
                .arg("gzip")
                .arg("--extract")
                .stdin(Stdio::from(File::open(tee2.path())?))
                .stdout(Stdio::from(File::create(tee3.path())?));
            extract.assert().success();
            tee3.assert(predicate::path::is_file());

            // Extract file to default filename
            let mut extract2 = Command::cargo_bin("cmprss")?;
            extract2
                .current_dir(&extract_dir)
                .arg("tar")
                .arg("--ignore-stdout")
                .arg("--extract")
                .stdin(Stdio::from(File::open(tee3.path())?));
            extract2.assert().success();

            // Assert the files are identical
            assert_files_equal(file.path(), &extract_dir.child("test.txt"));
            assert_files_equal(file2.path(), &extract_dir.child("test2.txt"));

            Ok(())
        }
    }
}
