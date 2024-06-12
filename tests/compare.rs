#[allow(dead_code)]
mod compare {
    use assert_cmd::prelude::*;
    use assert_fs::fixture::PathChild;
    use std::path::PathBuf;
    use std::process::Command;

    mod utils {
        use super::*;
        use rand::Rng;

        /// Asserts that the two directories are identical
        pub fn assert_directory(left: PathBuf, right: PathBuf) {
            let left_entries = std::fs::read_dir(left).unwrap();
            let right_entries = std::fs::read_dir(right).unwrap();

            // Fix the sorting of entries, as it's platform dependent
            let mut left_entries: Vec<_> = left_entries.map(|entry| entry.unwrap()).collect();
            let mut right_entries: Vec<_> = right_entries.map(|entry| entry.unwrap()).collect();
            left_entries.sort_by_key(|entry| entry.file_name());
            right_entries.sort_by_key(|entry| entry.file_name());

            for (left_entry, right_entry) in left_entries.iter().zip(right_entries) {
                assert_eq!(left_entry.file_name(), right_entry.file_name());

                let left_path = left_entry.path();
                let right_path = right_entry.path();

                if left_entry.file_type().unwrap().is_dir() {
                    assert!(right_entry.file_type().unwrap().is_dir());
                    assert_directory(left_path, right_path);
                } else {
                    let left_data = std::fs::read(&left_path).unwrap();
                    let right_data = std::fs::read(&right_path).unwrap();

                    assert_eq!(left_data, right_data);
                }
            }
        }

        /// Create a directory filled with random files
        pub fn create_random_files(dir: PathBuf, count: usize) {
            let mut rng = rand::thread_rng();
            std::fs::create_dir_all(&dir).unwrap();

            for i in 0..count {
                let path = format!("{}/file-{}", dir.display(), i);
                let data: Vec<u8> = (0..rng.gen_range(0..1000)).map(|_| rng.gen()).collect();
                std::fs::write(path, data).unwrap();
            }
        }
    }

    mod tar {
        use super::*;

        fn tar_dir(dir: PathBuf, archive: PathBuf) {
            // The current directory matters, as tar uses the relative path for the paths inside the tar file
            std::process::Command::new("tar")
                .arg("cf")
                .arg(archive.to_str().unwrap())
                .arg(dir.file_name().unwrap())
                .current_dir(dir.parent().unwrap())
                .output()
                .expect("Failed to execute command");
        }

        fn untar_archive(archive: PathBuf, output_dir: PathBuf) {
            std::process::Command::new("tar")
                .arg("xf")
                .arg(archive.to_str().unwrap())
                .arg("-C")
                .arg(output_dir.to_str().unwrap())
                .output()
                .expect("Failed to execute command");
        }

        fn cmprss_dir(dir: PathBuf, archive: PathBuf) {
            Command::cargo_bin("cmprss")
                .unwrap()
                .arg("tar")
                .arg(dir.to_str().unwrap())
                .arg(archive.to_str().unwrap())
                .assert()
                .success();
        }

        fn dcmprss_archive(archive: PathBuf, output_dir: PathBuf) {
            Command::cargo_bin("cmprss")
                .unwrap()
                .arg("tar")
                .arg("--extract")
                .arg(archive.to_str().unwrap())
                .arg(output_dir.to_str().unwrap())
                .assert()
                .success();
        }

        #[test]
        fn tar_tar() {
            let tmpdir = assert_fs::TempDir::new().unwrap();
            let starting_dir = tmpdir.child("orig").to_path_buf();
            let working_dir = assert_fs::TempDir::new().unwrap();
            let archive = working_dir.child("tar.tar").to_path_buf();

            utils::create_random_files(starting_dir.clone(), 10);

            // Run tar on the initial directory
            tar_dir(starting_dir.clone(), archive.clone());

            // Untar the tar file using tar
            let output_dir = assert_fs::TempDir::new().unwrap();
            untar_archive(archive, output_dir.to_path_buf());

            // Now compare the two directories
            utils::assert_directory(output_dir.to_path_buf(), tmpdir.to_path_buf());
        }

        #[test]
        fn cmprss_cmprss() {
            let tmpdir = assert_fs::TempDir::new().unwrap();
            let starting_dir = tmpdir.child("orig").to_path_buf();
            let working_dir = assert_fs::TempDir::new().unwrap();
            let archive = working_dir.child("tar.tar").to_path_buf();

            utils::create_random_files(starting_dir.clone(), 10);

            // Run tar on the initial directory
            cmprss_dir(starting_dir.clone(), archive.clone());

            // Untar the tar file using tar
            let output_dir = assert_fs::TempDir::new().unwrap();
            dcmprss_archive(archive, output_dir.to_path_buf());

            // Now compare the two directories
            utils::assert_directory(output_dir.to_path_buf(), tmpdir.to_path_buf());
        }

        #[test]
        fn cmprss_tar() {
            let tmpdir = assert_fs::TempDir::new().unwrap();
            let starting_dir = tmpdir.child("orig").to_path_buf();
            let working_dir = assert_fs::TempDir::new().unwrap();
            let archive = working_dir.child("tar.tar").to_path_buf();

            utils::create_random_files(starting_dir.clone(), 10);

            // Run tar on the initial directory
            cmprss_dir(starting_dir.clone(), archive.clone());

            // Untar the tar file using tar
            let output_dir = assert_fs::TempDir::new().unwrap();
            untar_archive(archive, output_dir.to_path_buf());

            // Now compare the two directories
            utils::assert_directory(output_dir.to_path_buf(), tmpdir.to_path_buf());
        }

        #[test]
        fn tar_cmprss() {
            let tmpdir = assert_fs::TempDir::new().unwrap();
            let starting_dir = tmpdir.child("orig").to_path_buf();
            let working_dir = assert_fs::TempDir::new().unwrap();
            let archive = working_dir.child("tar.tar").to_path_buf();

            utils::create_random_files(starting_dir.clone(), 10);

            // Run tar on the initial directory
            tar_dir(starting_dir.clone(), archive.clone());

            // Untar the tar file using tar
            let output_dir = assert_fs::TempDir::new().unwrap();
            dcmprss_archive(archive, output_dir.to_path_buf());

            // Now compare the two directories
            utils::assert_directory(output_dir.to_path_buf(), tmpdir.to_path_buf());
        }
    }

    // /// Tests the gzip comparisons
    // #[test]
    // #[cfg(feature = "interop")]
    // fn gzip() {
    //     test_with("gzip");
    // }

    // /// Tests the bzip2 comparisons
    // #[test]
    // #[cfg(feature = "interop")]
    // fn bzip2() {
    //     test_with("bzip2");
    // }

    // /// Tests the xz comparisons
    // #[test]
    // #[cfg(feature = "interop")]
    // fn xz() {
    //     test_with("xz");
    // }

    // /// Helper to spawn the script with any tool
    // fn test_with(tool: &str) {
    //     let output = std::process::Command::new("bin/test.sh")
    //         .arg(tool)
    //         .output()
    //         .expect("Failed to execute command");
    //     assert!(output.status.success());
    // }
}
