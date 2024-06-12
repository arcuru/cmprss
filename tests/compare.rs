/// Compare the interoperability of cmprss with the official tools.

#[allow(dead_code)]
mod compare {
    /// Tests the tar comparisons
    #[test]
    #[cfg(feature = "interop")]
    fn tar() {
        test_with("tar");
    }

    /// Tests the gzip comparisons
    #[test]
    #[cfg(feature = "interop")]
    fn gzip() {
        test_with("gzip");
    }

    /// Tests the bzip2 comparisons
    #[test]
    #[cfg(feature = "interop")]
    fn bzip2() {
        test_with("bzip2");
    }

    /// Tests the xz comparisons
    #[test]
    #[cfg(feature = "interop")]
    fn xz() {
        test_with("xz");
    }

    /// Helper to spawn the script with any tool
    fn test_with(tool: &str) {
        let output = std::process::Command::new("bin/test.sh")
            .arg(tool)
            .output()
            .expect("Failed to execute command");
        assert!(output.status.success());
    }
}
