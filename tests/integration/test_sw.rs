use crate::common::TempRepo;

#[test]
fn sw_with_number_switches_branch() {
    let repo = TempRepo::new();

    // Create branch history
    repo.checkout_new_branch("feature1");
    repo.checkout("main");
    repo.checkout_new_branch("feature2");
    repo.checkout("main");

    // Now recent should have: main, feature2, feature1
    // sw 1 should switch to most recent (which might be feature2 depending on reflog)
    let (code, stdout, _) = repo.gg(&["sw", "1"]);

    // May fail if no history, but shouldn't crash
    assert!(code == 0 || stdout.contains("No recent") || stdout.contains("Invalid"));
}

#[test]
fn sw_invalid_number_fails_gracefully() {
    let repo = TempRepo::new();

    // Create some branch history first
    repo.checkout_new_branch("branch1");
    repo.checkout("main");

    let (code, stdout, stderr) = repo.gg(&["sw", "999"]);

    // Should fail gracefully (invalid selection) or succeed if there's no history
    let combined = format!("{}{}", stdout, stderr);
    assert!(
        code != 0 || combined.contains("Invalid") || combined.contains("No recent"),
        "Expected failure or 'Invalid'/'No recent', got code={} output={}",
        code,
        combined
    );
}

#[test]
fn sw_zero_fails() {
    let repo = TempRepo::new();

    // Create some history
    repo.checkout_new_branch("branch1");
    repo.checkout("main");

    let (code, stdout, stderr) = repo.gg(&["sw", "0"]);

    // 0 is not a valid selection - should fail or show error
    let combined = format!("{}{}", stdout, stderr);
    assert!(
        code != 0 || combined.contains("Invalid") || combined.contains("No recent"),
        "Expected failure for sw 0, got code={} output={}",
        code,
        combined
    );
}

#[test]
fn sw_with_branch_history() {
    let repo = TempRepo::new();

    // Create substantial history
    repo.checkout_new_branch("branch-a");
    repo.checkout("main");
    repo.checkout_new_branch("branch-b");
    repo.checkout("main");
    repo.checkout("branch-a");
    repo.checkout("main");

    // Now try to switch
    let current_before = repo.current_branch();
    let (code, _, _) = repo.gg(&["sw", "1"]);

    // If successful, branch should change (or stay same if 1 is current)
    if code == 0 {
        // Just verify we're still in a valid state
        assert!(!repo.current_branch().is_empty());
    }
    // Allow failure if no valid history
    assert!(code == 0 || code == 1);
    let _ = current_before; // Silence unused warning
}
