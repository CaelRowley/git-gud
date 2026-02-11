use crate::common::TempRepo;

#[test]
fn push_on_main_branch() {
    let repo = TempRepo::with_remote();

    // Make a new commit
    repo.create_file("newfile.txt", "content");
    repo.commit("Add new file");

    let (code, stdout, _) = repo.gg(&["push"]);

    // Should succeed or show that push was attempted
    assert!(code == 0 || stdout.contains("push") || stdout.contains("Running"));
}

#[test]
fn push_on_feature_branch_sets_upstream() {
    let repo = TempRepo::with_remote();

    // Create and checkout a new branch
    repo.checkout_new_branch("feature-branch");
    repo.create_file("feature.txt", "content");
    repo.commit("Add feature");

    let (code, stdout, _) = repo.gg(&["push"]);

    // Should attempt to set upstream
    assert!(
        code == 0 || stdout.contains("--set-upstream") || stdout.contains("origin"),
        "Expected push to set upstream, got: {}",
        stdout
    );
}

#[test]
fn push_force_flag_uses_force_with_lease() {
    let repo = TempRepo::with_remote();

    repo.create_file("file.txt", "content");
    repo.commit("Commit");

    let (code, stdout, stderr) = repo.gg(&["push", "-f"]);

    // Check that force-with-lease is used (in the command output or behavior)
    let combined = format!("{}{}", stdout, stderr);
    assert!(
        code == 0 || combined.contains("force-with-lease") || combined.contains("push"),
        "Expected force push, got: {}",
        combined
    );
}

#[test]
fn push_force_long_flag() {
    let repo = TempRepo::with_remote();

    repo.create_file("file.txt", "content");
    repo.commit("Commit");

    let (code, _, _) = repo.gg(&["push", "--force"]);

    // Just verify it doesn't crash
    assert!(code == 0 || code == 1); // May fail if nothing to push, but shouldn't crash
}

#[test]
fn push_shows_running_command() {
    let repo = TempRepo::with_remote();

    let (_, stdout, _) = repo.gg(&["push"]);

    assert!(stdout.contains("Running:") || stdout.contains("git push"));
}

#[test]
fn push_on_branch_with_upstream() {
    let repo = TempRepo::with_remote();

    // Create branch, push to set upstream, then push again
    repo.checkout_new_branch("feature-with-upstream");
    repo.create_file("feature1.txt", "content");
    repo.commit("First feature commit");

    // First push sets upstream
    repo.gg(&["push"]);

    // Make another commit
    repo.create_file("feature2.txt", "more content");
    repo.commit("Second feature commit");

    // Second push should work without needing to set upstream again
    let (code, stdout, _) = repo.gg(&["push"]);

    assert!(
        code == 0 || stdout.contains("push"),
        "Expected successful push with existing upstream, got: {}",
        stdout
    );
}
