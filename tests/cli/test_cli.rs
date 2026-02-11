use assert_cmd::Command;
use predicates::prelude::*;

fn gg() -> Command {
    Command::new(env!("CARGO_BIN_EXE_gg"))
}

// =============================================================================
// Help and Version
// =============================================================================

#[test]
fn cli_help_shows_all_commands() {
    gg()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("status"))
        .stdout(predicate::str::contains("push"))
        .stdout(predicate::str::contains("sync"))
        .stdout(predicate::str::contains("quick-commit"))
        .stdout(predicate::str::contains("amend"))
        .stdout(predicate::str::contains("undo"))
        .stdout(predicate::str::contains("pr"))
        .stdout(predicate::str::contains("clean-branches"))
        .stdout(predicate::str::contains("recent"))
        .stdout(predicate::str::contains("sw"))
        .stdout(predicate::str::contains("today"))
        .stdout(predicate::str::contains("standup"));
}

#[test]
fn cli_version_works() {
    gg()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("gg"));
}

#[test]
fn cli_help_short_flag() {
    gg()
        .arg("-h")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage"));
}

#[test]
fn cli_version_short_flag() {
    gg()
        .arg("-V")
        .assert()
        .success()
        .stdout(predicate::str::contains("gg"));
}

// =============================================================================
// Command Aliases
// =============================================================================

#[test]
fn cli_status_alias_s() {
    gg()
        .arg("s")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Show status"));
}

#[test]
fn cli_push_alias_p() {
    gg()
        .arg("p")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Smart push"));
}

#[test]
fn cli_quick_commit_alias_qc() {
    gg()
        .arg("qc")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Quick commit"));
}

// =============================================================================
// Command Help
// =============================================================================

#[test]
fn cli_status_help() {
    gg()
        .args(["status", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("-s, --short"));
}

#[test]
fn cli_push_help() {
    gg()
        .args(["push", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("-f, --force"));
}

#[test]
fn cli_sync_help() {
    gg()
        .args(["sync", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--no-stash"));
}

#[test]
fn cli_quick_commit_help() {
    gg()
        .args(["quick-commit", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("-A, --all"))
        .stdout(predicate::str::contains("-p, --push"))
        .stdout(predicate::str::contains("MESSAGE"));
}

#[test]
fn cli_amend_help() {
    gg()
        .args(["amend", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("-a, --all"))
        .stdout(predicate::str::contains("-e, --edit"));
}

#[test]
fn cli_undo_help() {
    gg()
        .args(["undo", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--hard"))
        .stdout(predicate::str::contains("COUNT").or(predicate::str::contains("count")));
}

#[test]
fn cli_pr_help() {
    gg()
        .args(["pr", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("-p, --print"));
}

#[test]
fn cli_clean_branches_help() {
    gg()
        .args(["clean-branches", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("-f, --force"));
}

#[test]
fn cli_recent_help() {
    gg()
        .args(["recent", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("-c, --count"));
}

#[test]
fn cli_sw_help() {
    gg()
        .args(["sw", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("number"));
}

#[test]
fn cli_today_help() {
    gg()
        .args(["today", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("-a, --all"));
}

#[test]
fn cli_standup_help() {
    gg()
        .args(["standup", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("-a, --all"))
        .stdout(predicate::str::contains("-d, --days"));
}
