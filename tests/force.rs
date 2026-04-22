use assert_cmd::prelude::*;
use assert_fs::prelude::*;
use predicates::prelude::*;
use std::process::Command;

mod common;
use common::*;

mod force {
    use super::*;

    /// Without --force, cmprss must refuse to overwrite an existing -o target.
    ///
    /// ``` bash
    /// echo 'sentinel' > output.gz
    /// cmprss gzip -o output.gz input.txt   # fails, output.gz untouched
    /// ```
    #[test]
    fn refuses_overwrite_without_force() -> Result<(), Box<dyn std::error::Error>> {
        let working_dir = create_working_dir()?;
        let input = working_dir.child("input.txt");
        input.write_str("the real payload")?;

        let output = working_dir.child("output.gz");
        output.write_str("sentinel — must not be clobbered")?;

        let mut cmd = Command::cargo_bin("cmprss")?;
        cmd.current_dir(&working_dir)
            .args(["gzip", "-o", "output.gz", "input.txt"]);
        cmd.assert()
            .failure()
            .stderr(predicate::str::contains("already exists"));

        // The existing file must be byte-for-byte unchanged.
        output.assert("sentinel — must not be clobbered");
        Ok(())
    }

    /// With --force, cmprss overwrites an existing -o target.
    #[test]
    fn overwrites_with_force_explicit_output() -> Result<(), Box<dyn std::error::Error>> {
        let working_dir = create_working_dir()?;
        let input = working_dir.child("input.txt");
        input.write_str("the real payload")?;

        let output = working_dir.child("output.gz");
        output.write_str("sentinel — should be clobbered")?;

        let mut cmd = Command::cargo_bin("cmprss")?;
        cmd.current_dir(&working_dir)
            .args(["gzip", "--force", "-o", "output.gz", "input.txt"]);
        cmd.assert().success();

        // The file now contains real gzip output — confirm by round-tripping.
        let mut extract = Command::cargo_bin("cmprss")?;
        extract
            .current_dir(&working_dir)
            .args(["gzip", "--extract", "output.gz", "output.txt"]);
        extract.assert().success();
        working_dir.child("output.txt").assert("the real payload");
        Ok(())
    }

    /// With --force, a trailing existing file in the positional io_list is
    /// taken as the output and overwritten. Without --force, that trailing
    /// file gets mistakenly pulled into the input list.
    #[test]
    fn overwrites_with_force_positional_output() -> Result<(), Box<dyn std::error::Error>> {
        let working_dir = create_working_dir()?;
        let input = working_dir.child("input.txt");
        input.write_str("the real payload")?;

        let output = working_dir.child("input.txt.gz");
        output.write_str("stale archive")?;

        let mut cmd = Command::cargo_bin("cmprss")?;
        cmd.current_dir(&working_dir)
            .args(["gzip", "--force", "input.txt", "input.txt.gz"]);
        cmd.assert().success();

        // Round-trip the new archive to confirm it contains the fresh payload.
        let mut extract = Command::cargo_bin("cmprss")?;
        extract.current_dir(&working_dir).args([
            "gzip",
            "--extract",
            "input.txt.gz",
            "roundtrip.txt",
        ]);
        extract.assert().success();
        working_dir
            .child("roundtrip.txt")
            .assert("the real payload");
        Ok(())
    }
}
