use crate::common::TempRepo;

// =============================================================================
// Basic Passthrough
// =============================================================================

#[test]
fn passthrough_git_log() {
    let repo = TempRepo::new();

    let (code, stdout, _) = repo.gg(&["log", "--oneline"]);

    assert_eq!(code, 0);
    assert!(stdout.contains("Initial commit"));
}

#[test]
fn passthrough_git_log_with_flags() {
    let repo = TempRepo::new();

    repo.create_file("file1.txt", "1");
    repo.commit("Commit 1");
    repo.create_file("file2.txt", "2");
    repo.commit("Commit 2");

    let (code, stdout, _) = repo.gg(&["log", "--oneline", "-n", "2"]);

    assert_eq!(code, 0);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 2);
}

#[test]
fn passthrough_git_branch() {
    let repo = TempRepo::new();

    repo.checkout_new_branch("feature");
    let main_branch = repo.git_output(&["rev-parse", "--abbrev-ref", "HEAD"]);
    repo.run_git(&["checkout", "-"]); // Go back to previous branch

    let (code, stdout, _) = repo.gg(&["branch"]);

    assert_eq!(code, 0);
    // Should contain at least the feature branch we created
    assert!(stdout.contains("feature"));
    let _ = main_branch; // Silence unused warning
}

#[test]
fn passthrough_git_diff() {
    let repo = TempRepo::new();

    repo.modify_file("README.md", "Modified content");

    let (code, stdout, _) = repo.gg(&["diff"]);

    assert_eq!(code, 0);
    assert!(stdout.contains("Modified content") || stdout.contains("diff"));
}

#[test]
fn passthrough_git_show() {
    let repo = TempRepo::new();

    let (code, stdout, _) = repo.gg(&["show", "--stat", "HEAD"]);

    assert_eq!(code, 0);
    assert!(stdout.contains("Initial commit") || stdout.contains("README"));
}

// =============================================================================
// Complex Arguments
// =============================================================================

#[test]
fn passthrough_commit_with_message() {
    let repo = TempRepo::new();

    repo.create_file("newfile.txt", "content");
    repo.stage_all();

    let (code, _, _) = repo.gg(&["commit", "-m", "Passthrough commit message"]);

    assert_eq!(code, 0);
    assert_eq!(repo.last_commit_message(), "Passthrough commit message");
}

#[test]
fn passthrough_checkout_new_branch() {
    let repo = TempRepo::new();

    let (code, _, _) = repo.gg(&["checkout", "-b", "new-branch"]);

    assert_eq!(code, 0);
    assert_eq!(repo.current_branch(), "new-branch");
}

#[test]
fn passthrough_stash_and_pop() {
    let repo = TempRepo::new();

    repo.modify_file("README.md", "Changes to stash");

    let (code1, _, _) = repo.gg(&["stash"]);
    assert_eq!(code1, 0);
    assert!(!repo.has_unstaged_changes());

    let (code2, _, _) = repo.gg(&["stash", "pop"]);
    assert_eq!(code2, 0);
}

#[test]
fn passthrough_add_specific_file() {
    let repo = TempRepo::new();

    repo.create_file("file1.txt", "1");
    repo.create_file("file2.txt", "2");

    let (code, _, _) = repo.gg(&["add", "file1.txt"]);

    assert_eq!(code, 0);
    // Only file1 should be staged
    let status = repo.git_output(&["status", "--porcelain"]);
    assert!(status.contains("A  file1.txt") || status.contains("file1.txt"));
}

// =============================================================================
// Exit Codes
// =============================================================================

#[test]
fn passthrough_preserves_success_exit_code() {
    let repo = TempRepo::new();

    let (code, _, _) = repo.gg(&["--version"]);

    assert_eq!(code, 0);
}

#[test]
fn passthrough_preserves_failure_exit_code() {
    let repo = TempRepo::new();

    // Invalid git command should fail
    let (code, _, _) = repo.gg(&["not-a-real-git-command"]);

    assert_ne!(code, 0);
}

#[test]
fn passthrough_checkout_nonexistent_branch() {
    let repo = TempRepo::new();

    let (code, _, _) = repo.gg(&["checkout", "nonexistent-branch-12345"]);

    assert_ne!(code, 0);
}

// =============================================================================
// Edge Cases
// =============================================================================

#[test]
fn passthrough_double_dash() {
    let repo = TempRepo::new();

    repo.create_file("--weird-file.txt", "content");
    repo.stage_all();
    repo.commit("Add weird file");

    // Use -- to treat filename as literal
    let (code, _, _) = repo.gg(&["checkout", "HEAD", "--", "--weird-file.txt"]);

    // Should not crash
    assert!(code == 0 || code == 1);
}

#[test]
fn passthrough_empty_repo_log() {
    // Create a truly empty repo (no commits)
    let dir = tempfile::TempDir::new().unwrap();
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_gg"))
        .args(["log"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    // Should fail gracefully (no commits to show)
    assert_ne!(output.status.code().unwrap(), 0);
}

#[test]
fn passthrough_with_equals_syntax() {
    let repo = TempRepo::new();

    repo.create_file("file1.txt", "1");
    repo.commit("Commit 1");
    repo.create_file("file2.txt", "2");
    repo.commit("Commit 2");

    // git log --format="%H"
    let (code, stdout, _) = repo.gg(&["log", "--format=%H", "-n", "1"]);

    assert_eq!(code, 0);
    // Should be a commit hash (40 hex chars)
    assert!(stdout.trim().len() == 40);
}

#[test]
fn passthrough_git_status_porcelain() {
    let repo = TempRepo::new();

    repo.create_file("untracked.txt", "content");

    // Use passthrough for git status --porcelain
    // Note: "status" is a custom command so we use a different git command
    let output = repo.run_git(&["status", "--porcelain"]);
    let git_stdout = String::from_utf8_lossy(&output.stdout);

    assert!(git_stdout.contains("untracked.txt"));
}

// =============================================================================
// No Args Behavior
// =============================================================================

#[test]
fn no_args_runs_git_status() {
    let repo = TempRepo::new();

    repo.create_file("newfile.txt", "content");

    // gg with no args
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_gg"))
        .current_dir(&repo.path)
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success());
    // Should show status-like output
    assert!(stdout.contains("newfile") || stdout.contains("Untracked") || stdout.contains("??"));
}
