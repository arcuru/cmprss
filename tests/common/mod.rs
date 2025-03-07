use assert_fs::prelude::*;
use predicates::prelude::*;

pub fn create_test_file(
    name: &str,
    content: &str,
) -> Result<assert_fs::NamedTempFile, Box<dyn std::error::Error>> {
    let file = assert_fs::NamedTempFile::new(name)?;
    file.write_str(content)?;
    Ok(file)
}

pub fn create_working_dir() -> Result<assert_fs::TempDir, Box<dyn std::error::Error>> {
    Ok(assert_fs::TempDir::new()?)
}

#[allow(dead_code)]
pub fn create_persistent_working_dir() -> Result<assert_fs::TempDir, Box<dyn std::error::Error>> {
    Ok(assert_fs::TempDir::new()?.into_persistent())
}

pub fn assert_files_equal(file1: &std::path::Path, file2: &std::path::Path) {
    assert!(predicate::path::eq_file(file1).eval(file2));
}
