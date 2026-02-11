use crate::common::TempRepo;

#[test]
fn amend_basic() {
    let repo = TempRepo::new();

    // Stage a change
    repo.modify_file("README.md", "Modified for amend");
    repo.stage_all();

    let initial_count = repo.commit_count();
    let (code, stdout, _) = repo.gg(&["amend"]);

    assert_eq!(code, 0);
    // Should not create a new commit, just amend
    assert_eq!(repo.commit_count(), initial_count);
    assert!(stdout.contains("amend") || stdout.contains("Running"));
}

#[test]
fn amend_all_flag_stages_changes() {
    let repo = TempRepo::new();

    // Make changes without staging
    repo.modify_file("README.md", "Unstaged change");

    let initial_count = repo.commit_count();
    let (code, stdout, _) = repo.gg(&["amend", "-a"]);

    assert_eq!(code, 0);
    assert_eq!(repo.commit_count(), initial_count);
    // Should show add command
    assert!(stdout.contains("add") || stdout.contains("amend"));
}

#[test]
fn amend_all_long_flag() {
    let repo = TempRepo::new();

    repo.modify_file("README.md", "Change");

    let (code, _, _) = repo.gg(&["amend", "--all"]);

    assert_eq!(code, 0);
}

#[test]
fn amend_shows_running_commands() {
    let repo = TempRepo::new();

    repo.modify_file("README.md", "Change");
    repo.stage_all();

    let (_, stdout, _) = repo.gg(&["amend"]);

    assert!(stdout.contains("Running:") || stdout.contains("git"));
}

#[test]
fn amend_no_edit_by_default() {
    let repo = TempRepo::new();

    repo.modify_file("README.md", "Change");
    repo.stage_all();

    let (_, stdout, _) = repo.gg(&["amend"]);

    // Should use --no-edit by default
    assert!(stdout.contains("--no-edit"));
}

#[test]
fn amend_no_commits_fails() {
    // Create a truly empty repo (no commits)
    let dir = tempfile::TempDir::new().unwrap();
    std::process::Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    std::process::Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    std::process::Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_gg"))
        .args(["amend"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    // Should fail because there's nothing to amend
    assert_ne!(output.status.code().unwrap(), 0);
}
