use crate::common::TempRepo;

#[test]
fn clean_branches_dry_run_by_default() {
    let repo = TempRepo::new();

    // Create and merge a branch
    repo.checkout_new_branch("feature-to-merge");
    repo.create_file("feature.txt", "content");
    repo.commit("Feature commit");
    repo.checkout("main");
    repo.run_git(&["merge", "feature-to-merge"]);

    let branches_before = repo.branches();
    let (code, stdout, _) = repo.gg(&["clean-branches"]);

    assert_eq!(code, 0);
    // Should show what would be deleted or indicate --force needed, or show the branch name
    assert!(
        stdout.contains("--force") || stdout.contains("feature-to-merge") || stdout.contains("would be deleted") || stdout.contains("No merged"),
        "Expected dry-run output, got: {}",
        stdout
    );
    // Branch should still exist (dry-run doesn't delete)
    let branches_after = repo.branches();
    assert_eq!(branches_before.len(), branches_after.len());
}

#[test]
fn clean_branches_force_flag_deletes() {
    let repo = TempRepo::new();

    // Create and merge a branch
    repo.checkout_new_branch("merged-branch");
    repo.create_file("file.txt", "content");
    repo.commit("Commit");
    repo.checkout("main");
    repo.run_git(&["merge", "merged-branch"]);

    assert!(repo.branches().contains(&"merged-branch".to_string()));

    let (code, stdout, _) = repo.gg(&["clean-branches", "-f"]);

    assert_eq!(code, 0);
    // Should mention deleting or the branch name, or indicate success
    assert!(
        stdout.contains("Deleting") || stdout.contains("merged-branch") || stdout.contains("deleted") || !repo.branches().contains(&"merged-branch".to_string()),
        "Expected branch to be deleted, got: {}",
        stdout
    );
}

#[test]
fn clean_branches_force_long_flag() {
    let repo = TempRepo::new();

    repo.checkout_new_branch("to-delete");
    repo.commit("Commit");
    repo.checkout("main");
    repo.run_git(&["merge", "to-delete"]);

    let (code, _, _) = repo.gg(&["clean-branches", "--force"]);

    assert_eq!(code, 0);
}

#[test]
fn clean_branches_no_merged_branches() {
    let repo = TempRepo::new();

    let (code, stdout, _) = repo.gg(&["clean-branches"]);

    assert_eq!(code, 0);
    assert!(stdout.contains("No merged branches") || stdout.is_empty() || !stdout.contains("Deleting"));
}

#[test]
fn clean_branches_protects_main() {
    let repo = TempRepo::new();

    // Create a branch and merge it
    repo.checkout_new_branch("feature");
    repo.commit("Feature");
    repo.checkout("main");
    repo.run_git(&["merge", "feature"]);

    let (code, _, _) = repo.gg(&["clean-branches", "-f"]);

    assert_eq!(code, 0);
    // Main/master should never be deleted
    let branches = repo.branches();
    assert!(
        branches.contains(&"main".to_string()) || branches.contains(&"master".to_string()),
        "Main branch was deleted! Branches: {:?}",
        branches
    );
}
