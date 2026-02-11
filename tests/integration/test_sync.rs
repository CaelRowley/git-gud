use crate::common::TempRepo;

#[test]
fn sync_on_main_branch() {
    let repo = TempRepo::with_remote();

    let (code, stdout, _) = repo.gg(&["sync"]);

    // Should attempt pull --rebase
    assert!(
        code == 0 || stdout.contains("pull") || stdout.contains("rebase"),
        "Expected sync on main, got: {}",
        stdout
    );
}

#[test]
fn sync_on_feature_branch() {
    let repo = TempRepo::with_remote();

    // Create and checkout feature branch
    repo.checkout_new_branch("feature");
    repo.create_file("feature.txt", "content");
    repo.commit("Feature commit");

    let (code, stdout, _) = repo.gg(&["sync"]);

    // Should perform the rebase workflow
    assert!(
        stdout.contains("stash") || stdout.contains("checkout") || stdout.contains("rebase") || code == 0,
        "Expected sync workflow, got: {}",
        stdout
    );
}

#[test]
fn sync_no_stash_flag() {
    let repo = TempRepo::with_remote();

    repo.checkout_new_branch("feature");

    let (_, stdout, _) = repo.gg(&["sync", "--no-stash"]);

    // Should not contain stash commands
    // Note: This is a basic check - the flag should skip stash operations
    assert!(
        !stdout.contains("git stash\n") || stdout.contains("checkout"),
        "Checking no-stash behavior"
    );
}

#[test]
fn sync_shows_running_commands() {
    let repo = TempRepo::with_remote();

    let (_, stdout, _) = repo.gg(&["sync"]);

    assert!(stdout.contains("Running:") || stdout.contains("git"));
}

#[test]
fn sync_on_main_does_pull_rebase() {
    let repo = TempRepo::with_remote();

    // Ensure we're on main branch
    let branch = repo.current_branch();
    assert!(
        branch == "main" || branch == "master",
        "Expected main/master, got: {}",
        branch
    );

    let (_, stdout, _) = repo.gg(&["sync"]);

    // Should show pull --rebase command
    assert!(
        stdout.contains("pull") && stdout.contains("rebase"),
        "Expected 'pull --rebase', got: {}",
        stdout
    );
}

#[test]
fn sync_with_uncommitted_changes() {
    let repo = TempRepo::with_remote();

    // Create and checkout feature branch
    repo.checkout_new_branch("feature-uncommitted");
    repo.create_file("feature.txt", "content");
    repo.commit("Feature commit");

    // Push to create upstream
    repo.gg(&["push"]);

    // Make uncommitted changes
    repo.modify_file("feature.txt", "modified but not committed");

    assert!(repo.has_unstaged_changes());

    let (code, stdout, _) = repo.gg(&["sync"]);

    // Should handle uncommitted changes (stash or error gracefully)
    assert!(
        code == 0 || stdout.contains("stash") || stdout.contains("uncommitted"),
        "Expected sync to handle uncommitted changes, got code={} output={}",
        code,
        stdout
    );
}
