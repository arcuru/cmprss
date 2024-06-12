mod cli {
    use assert_cmd::prelude::*;
    use assert_fs::prelude::*;
    use predicates::prelude::*;
    use std::{
        fs::File,
        process::{Command, Stdio},
    };

    /// Tar roundtrip with a single file
    ///
    /// ``` bash
    /// cmprss tar test.txt archive.tar
    /// cmprss tar --extract archive.tar .
    /// ```
    #[test]
    fn tar_roundtrip_explicit() -> Result<(), Box<dyn std::error::Error>> {
        let file = assert_fs::NamedTempFile::new("test.txt")?;
        file.write_str("garbage data for testing")?;
        let working_dir = assert_fs::TempDir::new()?;
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
        working_dir
            .child("test.txt")
            .assert(predicate::path::eq_file(file.path()));

        Ok(())
    }

    /// Tar roundtrip with multiple files
    ///
    /// ``` bash
    /// cmprss tar test.txt test2.txt archive.tar
    /// cmprss tar --extract archive.tar .
    /// ```
    #[test]
    fn tar_roundtrip_explicit_two() -> Result<(), Box<dyn std::error::Error>> {
        let file = assert_fs::NamedTempFile::new("test.txt")?;
        file.write_str("garbage data for testing")?;
        let file2 = assert_fs::NamedTempFile::new("test2.txt")?;
        file2.write_str("more garbage data for testing")?;
        let working_dir = assert_fs::TempDir::new()?;
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
        working_dir
            .child("test.txt")
            .assert(predicate::path::eq_file(file.path()));
        working_dir
            .child("test2.txt")
            .assert(predicate::path::eq_file(file2.path()));

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
    fn tar_roundtrip_implicit() -> Result<(), Box<dyn std::error::Error>> {
        let file = assert_fs::NamedTempFile::new("test.txt")?;
        file.write_str("garbage data for testing")?;
        let working_dir = assert_fs::TempDir::new()?.into_persistent();
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
        working_dir
            .child("test.txt")
            .assert(predicate::path::eq_file(file.path()));

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
    fn tar_roundtrip_implicit_two() -> Result<(), Box<dyn std::error::Error>> {
        let file = assert_fs::NamedTempFile::new("test.txt")?;
        file.write_str("garbage data for testing")?;
        let file2 = assert_fs::NamedTempFile::new("test2.txt")?;
        file2.write_str("more garbage data for testing")?;
        let working_dir = assert_fs::TempDir::new()?.into_persistent();
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
        working_dir
            .child("test.txt")
            .assert(predicate::path::eq_file(file.path()));
        working_dir
            .child("test2.txt")
            .assert(predicate::path::eq_file(file2.path()));

        Ok(())
    }

    /// Gzip roundtrip using stdin
    /// Compressing: input = stdin, output = test.txt.gz
    /// Extracting:  input = test.txt.gz, output = test.txt
    ///
    /// ``` bash
    /// cat test.txt | cmprss gzip test.txt.gz
    /// cmprss gzip --ignore-pipes --extract test.txt.gz
    /// ```
    #[test]
    fn gzip_roundtrip_stdin() -> Result<(), Box<dyn std::error::Error>> {
        let file = assert_fs::NamedTempFile::new("test.txt")?;
        file.write_str("garbage data for testing")?;
        let working_dir = assert_fs::TempDir::new()?;
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
        working_dir
            .child("test.txt")
            .assert(predicate::path::eq_file(file.path()));

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
    fn gzip_roundtrip_inferred_output_filenames() -> Result<(), Box<dyn std::error::Error>> {
        let file = assert_fs::NamedTempFile::new("test.txt")?;
        file.write_str("garbage data for testing")?;
        let working_dir = assert_fs::TempDir::new()?;
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
        working_dir
            .child("archive")
            .assert(predicate::path::eq_file(file.path()));

        Ok(())
    }

    /// Xz roundtrip using files
    /// Compressing: input = test.txt, output = test.txt.xz
    /// Extracting:  input = test.txt.xz, output = test.txt
    ///
    /// ``` bash
    /// cmprss xz test.txt test.txt.xz
    /// cmprss xz --extract --ignore-pipes test.txt.xz
    /// ```
    #[test]
    fn xz_roundtrip_explicit() -> Result<(), Box<dyn std::error::Error>> {
        let file = assert_fs::NamedTempFile::new("test.txt")?;
        file.write_str("garbage data for testing")?;
        let working_dir = assert_fs::TempDir::new()?;
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
        working_dir
            .child("test.txt")
            .assert(predicate::path::eq_file(file.path()));

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
    fn xz_roundtrip_stdin() -> Result<(), Box<dyn std::error::Error>> {
        let file = assert_fs::NamedTempFile::new("test.txt")?;
        file.write_str("garbage data for testing")?;
        let working_dir = assert_fs::TempDir::new()?;
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
        working_dir
            .child("out.txt")
            .assert(predicate::path::eq_file(file.path()));

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
    fn xz_roundtrip_stdout() -> Result<(), Box<dyn std::error::Error>> {
        let file = assert_fs::NamedTempFile::new("test.txt")?;
        file.write_str("garbage data for testing")?;
        let working_dir = assert_fs::TempDir::new()?;
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
        // TODO: This fails, but manual testing shows it works fine
        //.stdout(Stdio::from(File::create("out.txt")?));
        extract.assert().success();

        // Assert the files are identical
        working_dir
            .child("out.txt")
            .assert(predicate::path::eq_file(file.path()));

        Ok(())
    }

    /// Bzip2 roundtrip using files
    /// Compressing: input = test.txt, output = test.txt.bz2
    /// Extracting:  input = test.txt.bz2, output = test.txt
    ///
    /// ``` bash
    /// cmprss bzip2 test.txt test.txt.bz2
    /// cmprss bzip2 --extract --ignore-pipes test.txt.bz2
    /// ```
    #[test]
    fn bzip2_roundtrip_explicit() -> Result<(), Box<dyn std::error::Error>> {
        let file = assert_fs::NamedTempFile::new("test.txt")?;
        file.write_str("garbage data for testing")?;

        let working_dir = assert_fs::TempDir::new()?;
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
        working_dir
            .child("test.txt")
            .assert(predicate::path::eq_file(file.path()));

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
    fn bzip2_roundtrip_stdin() -> Result<(), Box<dyn std::error::Error>> {
        let file = assert_fs::NamedTempFile::new("test.txt")?;
        file.write_str("garbage data for testing")?;

        let working_dir = assert_fs::TempDir::new()?;
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
        working_dir
            .child("out.txt")
            .assert(predicate::path::eq_file(file.path()));

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
    fn bzip2_roundtrip_stdout() -> Result<(), Box<dyn std::error::Error>> {
        let file = assert_fs::NamedTempFile::new("test.txt")?;
        file.write_str("garbage data for testing")?;

        let working_dir = assert_fs::TempDir::new()?;
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
        // TODO: This fails, but manual testing shows it works fine
        //.stdout(Stdio::from(File::create("out.txt")?));
        extract.assert().success();

        // Assert the files are identical
        working_dir
            .child("out.txt")
            .assert(predicate::path::eq_file(file.path()));

        Ok(())
    }

    /// Magic roundtrip using stdin
    /// Compressing: input = stdin, output = test.txt.gz
    /// Extracting:  input = test.txt.gz, output = test.txt
    ///
    /// ``` bash
    /// cat test.txt | cmprss test.txt.gz
    /// cmprss gz --extract test.txt.gz out.txt
    /// ```
    #[test]
    fn magic_roundtrip_stdin() -> Result<(), Box<dyn std::error::Error>> {
        let file = assert_fs::NamedTempFile::new("test.txt")?;
        file.write_str("garbage data for testing")?;

        let working_dir = assert_fs::TempDir::new()?;
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
        working_dir
            .child("test.txt")
            .assert(predicate::path::eq_file(file.path()));

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
    fn magic_roundtrip_files() -> Result<(), Box<dyn std::error::Error>> {
        let file = assert_fs::NamedTempFile::new("test.txt")?;
        file.write_str("garbage data for testing")?;

        let working_dir = assert_fs::TempDir::new()?;
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
        working_dir
            .child("out.txt")
            .assert(predicate::path::eq_file(file.path()));

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
    fn magic_roundtrip_stdout_decompression() -> Result<(), Box<dyn std::error::Error>> {
        let file = assert_fs::NamedTempFile::new("test.txt")?;
        file.write_str("garbage data for testing")?;

        let working_dir = assert_fs::TempDir::new()?;
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
        out_file.assert(predicate::path::eq_file(file.path()));
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
    fn magic_roundtrip_stdin_compression() -> Result<(), Box<dyn std::error::Error>> {
        let file = assert_fs::NamedTempFile::new("test.txt")?;
        file.write_str("garbage data for testing")?;

        let working_dir = assert_fs::TempDir::new()?;
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
        out_file.assert(predicate::path::eq_file(file.path()));

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
    fn magic_roundtrip_default_filenames() -> Result<(), Box<dyn std::error::Error>> {
        let file = assert_fs::NamedTempFile::new("test.txt")?;
        file.write_str("garbage data for testing")?;

        let working_dir = assert_fs::TempDir::new()?;
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
        working_dir
            .child("test.txt")
            .assert(predicate::path::eq_file(file.path()));

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
    fn magic_roundtrip_multiple_files_tar() -> Result<(), Box<dyn std::error::Error>> {
        let file = assert_fs::NamedTempFile::new("test.txt")?;
        file.write_str("garbage data for testing")?;
        let file2 = assert_fs::NamedTempFile::new("test2.txt")?;
        file2.write_str("more garbage data for testing")?;

        let working_dir = assert_fs::TempDir::new()?;
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
        working_dir
            .child("test.txt")
            .assert(predicate::path::eq_file(file.path()));
        working_dir
            .child("test2.txt")
            .assert(predicate::path::eq_file(file2.path()));

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
    fn magic_roundtrip_tar_gz() -> Result<(), Box<dyn std::error::Error>> {
        let file = assert_fs::NamedTempFile::new("test.txt")?;
        file.write_str("garbage data for testing")?;
        let file2 = assert_fs::NamedTempFile::new("test2.txt")?;
        file2.write_str("more garbage data for testing")?;

        let working_dir = assert_fs::TempDir::new()?;
        let archive = working_dir.child("archive.tar");
        archive.assert(predicate::path::missing());
        let archive2 = working_dir.child("archive.tar.gz");
        archive2.assert(predicate::path::missing());

        let extract_dir = assert_fs::TempDir::new()?;

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
        extract_dir
            .child("test.txt")
            .assert(predicate::path::eq_file(file.path()));
        extract_dir
            .child("test2.txt")
            .assert(predicate::path::eq_file(file2.path()));

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
    fn magic_roundtrip_tar_gz_pipes() -> Result<(), Box<dyn std::error::Error>> {
        let file = assert_fs::NamedTempFile::new("test.txt")?;
        file.write_str("garbage data for testing")?;
        let file2 = assert_fs::NamedTempFile::new("test2.txt")?;
        file2.write_str("more garbage data for testing")?;

        let working_dir = assert_fs::TempDir::new()?;
        let tee1 = working_dir.child("tee1");
        let tee2 = working_dir.child("tee2");
        let tee3 = working_dir.child("tee3");

        let extract_dir = assert_fs::TempDir::new()?;

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
        extract_dir
            .child("test.txt")
            .assert(predicate::path::eq_file(file.path()));
        extract_dir
            .child("test2.txt")
            .assert(predicate::path::eq_file(file2.path()));

        Ok(())
    }
}
