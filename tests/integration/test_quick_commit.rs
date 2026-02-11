use crate::common::TempRepo;

#[test]
fn qc_basic_commits_tracked_changes() {
    let repo = TempRepo::new();
    let initial_count = repo.commit_count();

    // Modify a tracked file
    repo.modify_file("README.md", "Modified content");

    let (code, _, _) = repo.gg(&["quick-commit", "Test commit"]);

    assert_eq!(code, 0);
    assert_eq!(repo.commit_count(), initial_count + 1);
    assert_eq!(repo.last_commit_message(), "Test commit");
}

#[test]
fn qc_alias_works() {
    let repo = TempRepo::new();
    let initial_count = repo.commit_count();

    repo.modify_file("README.md", "Modified");

    let (code, _, _) = repo.gg(&["qc", "Using alias"]);

    assert_eq!(code, 0);
    assert_eq!(repo.commit_count(), initial_count + 1);
}

#[test]
fn qc_all_flag_includes_untracked() {
    let repo = TempRepo::new();
    let initial_count = repo.commit_count();

    // Create a new untracked file
    repo.create_file("newfile.txt", "content");

    // Without -A, it should not commit untracked files with `add -u`
    // But with -A, it should
    let (code, _, _) = repo.gg(&["qc", "-A", "Commit with all"]);

    assert_eq!(code, 0);
    assert_eq!(repo.commit_count(), initial_count + 1);
    assert!(!repo.has_untracked_files());
}

#[test]
fn qc_all_long_flag() {
    let repo = TempRepo::new();
    let initial_count = repo.commit_count();

    repo.create_file("another.txt", "content");

    let (code, _, _) = repo.gg(&["qc", "--all", "With --all"]);

    assert_eq!(code, 0);
    assert_eq!(repo.commit_count(), initial_count + 1);
}

#[test]
fn qc_push_flag() {
    let repo = TempRepo::with_remote();
    let initial_count = repo.commit_count();

    repo.modify_file("README.md", "Modified for push");

    let (code, stdout, _) = repo.gg(&["qc", "-p", "Commit and push"]);

    assert_eq!(code, 0);
    assert_eq!(repo.commit_count(), initial_count + 1);
    // Should show push command
    assert!(stdout.contains("push") || stdout.contains("git push"));
}

#[test]
fn qc_all_and_push_combined() {
    let repo = TempRepo::with_remote();
    let initial_count = repo.commit_count();

    repo.create_file("combined.txt", "content");

    let (code, stdout, _) = repo.gg(&["qc", "-Ap", "All and push"]);

    assert_eq!(code, 0);
    assert_eq!(repo.commit_count(), initial_count + 1);
    assert!(stdout.contains("push"));
}

#[test]
fn qc_no_changes_fails() {
    let repo = TempRepo::new();
    let initial_count = repo.commit_count();

    // No changes to commit
    let (code, _, _) = repo.gg(&["qc", "Nothing to commit"]);

    // Should fail because there's nothing to commit
    assert_ne!(code, 0);
    assert_eq!(repo.commit_count(), initial_count);
}

#[test]
fn qc_shows_running_commands() {
    let repo = TempRepo::new();
    repo.modify_file("README.md", "Change");

    let (_, stdout, _) = repo.gg(&["qc", "Test"]);

    assert!(stdout.contains("Running:") || stdout.contains("git"));
}
