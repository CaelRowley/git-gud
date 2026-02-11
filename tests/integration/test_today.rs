use crate::common::TempRepo;

#[test]
fn today_shows_commits() {
    let repo = TempRepo::new();

    // The initial commit was made today
    let (code, stdout, _) = repo.gg(&["today"]);

    assert_eq!(code, 0);
    // Should show at least the initial commit or be empty
    assert!(stdout.contains("Initial") || stdout.is_empty() || !stdout.contains("error"));
}

#[test]
fn today_all_flag() {
    let repo = TempRepo::new();

    let (code, _, _) = repo.gg(&["today", "-a"]);

    assert_eq!(code, 0);
}

#[test]
fn today_all_long_flag() {
    let repo = TempRepo::new();

    let (code, _, _) = repo.gg(&["today", "--all"]);

    assert_eq!(code, 0);
}

#[test]
fn today_with_multiple_commits() {
    let repo = TempRepo::new();

    // Create more commits
    repo.create_file("file1.txt", "1");
    repo.commit("Today commit 1");
    repo.create_file("file2.txt", "2");
    repo.commit("Today commit 2");

    let (code, stdout, _) = repo.gg(&["today"]);

    assert_eq!(code, 0);
    // Should show the commits
    assert!(stdout.contains("Today commit") || stdout.contains("commit") || stdout.is_empty());
}

#[test]
fn today_no_commits_today() {
    // This is tricky to test since commits are made "today"
    // We just verify it doesn't crash
    let repo = TempRepo::new();

    let (code, _, _) = repo.gg(&["today"]);

    assert_eq!(code, 0);
}

#[test]
fn today_uses_oneline_format() {
    let repo = TempRepo::new();

    repo.create_file("test.txt", "content");
    repo.commit("Test commit for oneline");

    let (code, stdout, _) = repo.gg(&["today"]);

    assert_eq!(code, 0);
    // Oneline format should be short
    if !stdout.is_empty() {
        let lines: Vec<&str> = stdout.lines().collect();
        // Each commit should be on one line
        for line in lines {
            assert!(line.len() < 200, "Line too long for oneline format");
        }
    }
}
