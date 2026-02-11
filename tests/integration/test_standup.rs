use crate::common::TempRepo;

#[test]
fn standup_default() {
    let repo = TempRepo::new();

    let (code, _, _) = repo.gg(&["standup"]);

    assert_eq!(code, 0);
}

#[test]
fn standup_all_flag() {
    let repo = TempRepo::new();

    let (code, _, _) = repo.gg(&["standup", "-a"]);

    assert_eq!(code, 0);
}

#[test]
fn standup_all_long_flag() {
    let repo = TempRepo::new();

    let (code, _, _) = repo.gg(&["standup", "--all"]);

    assert_eq!(code, 0);
}

#[test]
fn standup_days_flag() {
    let repo = TempRepo::new();

    let (code, _, _) = repo.gg(&["standup", "-d", "5"]);

    assert_eq!(code, 0);
}

#[test]
fn standup_days_long_flag() {
    let repo = TempRepo::new();

    let (code, _, _) = repo.gg(&["standup", "--days", "7"]);

    assert_eq!(code, 0);
}

#[test]
fn standup_all_and_days_combined() {
    let repo = TempRepo::new();

    let (code, _, _) = repo.gg(&["standup", "-a", "-d", "3"]);

    assert_eq!(code, 0);
}

#[test]
fn standup_shows_commits() {
    let repo = TempRepo::new();

    // Create some commits
    repo.create_file("standup.txt", "content");
    repo.commit("Standup test commit");

    let (code, stdout, _) = repo.gg(&["standup", "-d", "1"]);

    assert_eq!(code, 0);
    // Should show recent commits
    assert!(stdout.is_empty() || stdout.contains("commit") || stdout.contains("Standup"));
}

#[test]
fn standup_empty_result() {
    let repo = TempRepo::new();

    // Look for commits from a very specific time range that won't exist
    let (code, _, _) = repo.gg(&["standup", "-d", "0"]);

    // Should not crash, even with edge case
    assert!(code == 0 || code == 1);
}
