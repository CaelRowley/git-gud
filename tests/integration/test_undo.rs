use crate::common::TempRepo;

#[test]
fn undo_default_undoes_one_commit() {
    let repo = TempRepo::new();

    // Create additional commits
    repo.create_file("file1.txt", "content");
    repo.commit("Commit 1");
    repo.create_file("file2.txt", "content");
    repo.commit("Commit 2");

    let count_before = repo.commit_count();
    let (code, stdout, _) = repo.gg(&["undo"]);

    assert_eq!(code, 0);
    assert_eq!(repo.commit_count(), count_before - 1);
    // Should use soft reset
    assert!(stdout.contains("reset") || stdout.contains("Running"));
}

#[test]
fn undo_specific_count() {
    let repo = TempRepo::new();

    // Create commits
    repo.create_file("file1.txt", "1");
    repo.commit("Commit 1");
    repo.create_file("file2.txt", "2");
    repo.commit("Commit 2");
    repo.create_file("file3.txt", "3");
    repo.commit("Commit 3");

    let count_before = repo.commit_count();
    let (code, _, _) = repo.gg(&["undo", "2"]);

    assert_eq!(code, 0);
    assert_eq!(repo.commit_count(), count_before - 2);
}

#[test]
fn undo_hard_flag() {
    let repo = TempRepo::new();

    repo.create_file("file.txt", "content");
    repo.commit("To be undone");

    let count_before = repo.commit_count();
    let (code, stdout, _) = repo.gg(&["undo", "--hard"]);

    assert_eq!(code, 0);
    assert_eq!(repo.commit_count(), count_before - 1);
    assert!(stdout.contains("--hard") || stdout.contains("reset"));
}

#[test]
fn undo_preserves_changes_without_hard() {
    let repo = TempRepo::new();

    // Create a file and commit
    repo.create_file("important.txt", "important content");
    repo.commit("Important commit");

    let (code, _, _) = repo.gg(&["undo"]);

    assert_eq!(code, 0);
    // Changes should be staged (soft reset)
    assert!(repo.has_staged_changes() || std::fs::read_to_string(repo.path.join("important.txt")).is_ok());
}

#[test]
fn undo_shows_running_command() {
    let repo = TempRepo::new();

    repo.create_file("file.txt", "content");
    repo.commit("Commit");

    let (_, stdout, _) = repo.gg(&["undo"]);

    assert!(stdout.contains("Running:") || stdout.contains("git reset"));
}

#[test]
fn undo_with_count_and_hard() {
    let repo = TempRepo::new();

    repo.create_file("file1.txt", "1");
    repo.commit("Commit 1");
    repo.create_file("file2.txt", "2");
    repo.commit("Commit 2");

    let count_before = repo.commit_count();
    let (code, _, _) = repo.gg(&["undo", "2", "--hard"]);

    assert_eq!(code, 0);
    assert_eq!(repo.commit_count(), count_before - 2);
}

#[test]
fn undo_no_commits_fails() {
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
        .args(["undo"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    // Should fail because there's nothing to undo
    assert_ne!(output.status.code().unwrap(), 0);
}
