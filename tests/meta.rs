use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;

mod common;

/// The hidden `completions` / `manpage` subcommands are integration-tested by
/// shape, not content: each must succeed and emit non-empty stdout so that
/// packagers can pipe the output into a file during install.
mod meta {
    use super::*;

    #[test]
    fn completions_bash() -> Result<(), Box<dyn std::error::Error>> {
        let mut cmd = Command::cargo_bin("cmprss")?;
        cmd.args(["completions", "bash"]);
        cmd.assert()
            .success()
            .stdout(predicate::str::contains("_cmprss"));
        Ok(())
    }

    #[test]
    fn completions_zsh() -> Result<(), Box<dyn std::error::Error>> {
        let mut cmd = Command::cargo_bin("cmprss")?;
        cmd.args(["completions", "zsh"]);
        cmd.assert()
            .success()
            .stdout(predicate::str::contains("#compdef cmprss"));
        Ok(())
    }

    #[test]
    fn manpage_emits_troff() -> Result<(), Box<dyn std::error::Error>> {
        let mut cmd = Command::cargo_bin("cmprss")?;
        cmd.arg("manpage");
        cmd.assert()
            .success()
            .stdout(predicate::str::contains(".TH cmprss 1"))
            .stdout(predicate::str::contains(".SH NAME"));
        Ok(())
    }

    /// `cmprss --help` must not advertise the hidden meta subcommands.
    #[test]
    fn meta_subcommands_hidden_from_help() -> Result<(), Box<dyn std::error::Error>> {
        let mut cmd = Command::cargo_bin("cmprss")?;
        cmd.arg("--help");
        cmd.assert()
            .success()
            .stdout(predicate::str::contains("completions").not())
            .stdout(predicate::str::contains("manpage").not());
        Ok(())
    }
}
