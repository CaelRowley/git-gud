use crate::common::TempRepo;

#[test]
fn status_clean_repo() {
    let repo = TempRepo::new();

    let (code, stdout, _) = repo.gg(&["status"]);

    assert_eq!(code, 0);
    assert!(stdout.contains("On branch:"));
    // Branch name could be main or master depending on git config
    assert!(stdout.contains("main") || stdout.contains("master") || stdout.contains("refs/heads"));
    assert!(stdout.contains("nothing to commit") || !stdout.contains("Changes"));
}

#[test]
fn status_staged_files() {
    let repo = TempRepo::new();

    // Create and stage a new file
    repo.create_file("new_file.txt", "content");
    repo.stage("new_file.txt");

    let (code, stdout, _) = repo.gg(&["status"]);

    assert_eq!(code, 0);
    assert!(stdout.contains("Changes to be committed:"));
    assert!(stdout.contains("new_file.txt"));
}

#[test]
fn status_unstaged_files() {
    let repo = TempRepo::new();

    // Modify an existing tracked file
    repo.modify_file("README.md", "# Modified content\n");

    let (code, stdout, _) = repo.gg(&["status"]);

    assert_eq!(code, 0);
    assert!(stdout.contains("Changes not staged for commit:"));
    assert!(stdout.contains("README.md"));
}

#[test]
fn status_untracked_files() {
    let repo = TempRepo::new();

    // Create a new untracked file
    repo.create_file("untracked.txt", "content");

    let (code, stdout, _) = repo.gg(&["status"]);

    assert_eq!(code, 0);
    assert!(stdout.contains("Untracked files:"));
    assert!(stdout.contains("untracked.txt"));
}

#[test]
fn status_deleted_files() {
    let repo = TempRepo::new();

    // Delete a tracked file
    repo.delete_file("README.md");

    let (code, stdout, _) = repo.gg(&["status"]);

    assert_eq!(code, 0);
    assert!(stdout.contains("Deleted files:") || stdout.contains("deleted:"));
}

#[test]
fn status_mixed_changes() {
    let repo = TempRepo::new();

    // Create multiple change types
    repo.create_file("staged.txt", "staged content");
    repo.stage("staged.txt");
    repo.create_file("untracked.txt", "untracked");
    repo.modify_file("README.md", "modified");

    let (code, stdout, _) = repo.gg(&["status"]);

    assert_eq!(code, 0);
    assert!(stdout.contains("Changes to be committed:"));
    assert!(stdout.contains("Untracked files:"));
    assert!(stdout.contains("Changes not staged for commit:"));
}

#[test]
fn status_short_flag_delegates_to_git() {
    let repo = TempRepo::new();

    // Create some changes
    repo.create_file("newfile.txt", "content");

    let (code, stdout, _) = repo.gg(&["status", "-s"]);

    assert_eq!(code, 0);
    // Short format uses ?? for untracked
    assert!(stdout.contains("??") || stdout.contains("newfile.txt"));
}

#[test]
fn status_short_flag_long_form() {
    let repo = TempRepo::new();

    repo.create_file("newfile.txt", "content");

    let (code, stdout, _) = repo.gg(&["status", "--short"]);

    assert_eq!(code, 0);
    assert!(stdout.contains("??") || stdout.contains("newfile.txt"));
}

#[test]
fn status_empty_repo() {
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
        .args(["status"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    // Empty repo may fail (no HEAD) or succeed - either is acceptable
    // The important thing is it doesn't panic/crash
    let code = output.status.code().unwrap_or(-1);
    assert!(
        code == 0 || code == 1,
        "Status should handle empty repo gracefully, got exit code: {}",
        code
    );
}
