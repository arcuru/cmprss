//! Shared helpers for container formats (tar, zip, 7z).
//!
//! Single-stream codecs know the total input size up front from file
//! metadata. Container formats take N paths and recurse into directories, so
//! they have to pre-walk the input to get a meaningful progress total.

use std::path::Path;

/// Sum sizes of all regular files reachable from the given paths, recursing
/// into directories. Uses `fs::metadata` (follows symlinks) so that the
/// file-vs-directory judgment matches `Path::is_file` / `Path::is_dir`
/// semantics used by each backend's walker — otherwise a symlink to a file
/// would contribute 0 to the total while the walker reads (and bar-ticks)
/// the full target. Best-effort: anything we can't stat (permission denied,
/// racy deletion, broken symlink) is counted as zero; the bar may finish
/// short rather than fail the run.
pub fn total_input_bytes<P: AsRef<Path>>(paths: &[P]) -> u64 {
    paths.iter().map(|p| sum_path(p.as_ref())).sum()
}

fn sum_path(path: &Path) -> u64 {
    let Ok(meta) = std::fs::metadata(path) else {
        return 0;
    };
    if meta.is_file() {
        return meta.len();
    }
    if meta.is_dir() {
        let Ok(entries) = std::fs::read_dir(path) else {
            return 0;
        };
        return entries.flatten().map(|e| sum_path(&e.path())).sum();
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_fs::prelude::*;

    #[test]
    fn sums_single_file() {
        let dir = assert_fs::TempDir::new().unwrap();
        let f = dir.child("a.txt");
        f.write_str("hello").unwrap();
        assert_eq!(total_input_bytes(&[f.path().to_path_buf()]), 5);
    }

    #[test]
    fn sums_directory_recursively() {
        let dir = assert_fs::TempDir::new().unwrap();
        dir.child("a.txt").write_str("abc").unwrap();
        dir.child("sub/b.txt").write_str("defgh").unwrap();
        assert_eq!(total_input_bytes(&[dir.path().to_path_buf()]), 8);
    }

    #[test]
    fn missing_path_counts_zero() {
        assert_eq!(
            total_input_bytes(&[std::path::PathBuf::from("/nope/xx")]),
            0
        );
    }

    /// Symlinks to regular files must contribute their target's size so the
    /// bar total matches what the walkers actually read. Regression for
    /// tar/zip/7z bars overshooting past 100% on directories containing
    /// symlinks.
    #[cfg(unix)]
    #[test]
    fn follows_symlink_to_file() {
        let dir = assert_fs::TempDir::new().unwrap();
        dir.child("target.txt").write_str("abcdefghij").unwrap();
        std::os::unix::fs::symlink(dir.path().join("target.txt"), dir.path().join("link.txt"))
            .unwrap();
        // target.txt (10) + link.txt following to target (10) = 20
        assert_eq!(total_input_bytes(&[dir.path().to_path_buf()]), 20);
    }
}
