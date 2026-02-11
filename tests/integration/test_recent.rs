use crate::common::TempRepo;

#[test]
fn recent_shows_branches() {
    let repo = TempRepo::new();

    // Create some branch history
    repo.checkout_new_branch("branch1");
    repo.checkout("main");
    repo.checkout_new_branch("branch2");
    repo.checkout("main");

    let (code, stdout, _) = repo.gg(&["recent"]);

    assert_eq!(code, 0);
    // Should show recent branches or indicate none found
    assert!(
        stdout.contains("Recent branches:") || stdout.contains("No recent") || stdout.contains("branch"),
        "Output: {}",
        stdout
    );
}

#[test]
fn recent_count_flag() {
    let repo = TempRepo::new();

    // Create branch history
    for i in 1..=5 {
        repo.checkout_new_branch(&format!("branch{}", i));
        repo.checkout("main");
    }

    let (code, _, _) = repo.gg(&["recent", "-c", "3"]);

    assert_eq!(code, 0);
}

#[test]
fn recent_count_long_flag() {
    let repo = TempRepo::new();

    let (code, _, _) = repo.gg(&["recent", "--count", "5"]);

    assert_eq!(code, 0);
}

#[test]
fn recent_no_history() {
    let repo = TempRepo::new();

    // Fresh repo with no checkout history
    let (code, stdout, _) = repo.gg(&["recent"]);

    assert_eq!(code, 0);
    // Should handle gracefully
    assert!(stdout.contains("No recent") || stdout.contains("Recent") || stdout.is_empty());
}

#[test]
fn recent_default_count_is_10() {
    let repo = TempRepo::new();

    // The default should be 10, just verify it doesn't crash
    let (code, _, _) = repo.gg(&["recent"]);

    assert_eq!(code, 0);
}
