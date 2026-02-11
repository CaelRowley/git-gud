use crate::common::TempRepo;

#[test]
fn pr_print_flag_outputs_url() {
    let repo = TempRepo::new();

    // Add a GitHub-style remote
    repo.run_git(&["remote", "add", "origin", "git@github.com:user/repo.git"]);

    let (code, stdout, _) = repo.gg(&["pr", "-p"]);

    assert_eq!(code, 0);
    assert!(stdout.contains("github.com"));
    assert!(stdout.contains("compare") || stdout.contains("pull"));
}

#[test]
fn pr_print_long_flag() {
    let repo = TempRepo::new();

    repo.run_git(&["remote", "add", "origin", "https://github.com/user/repo.git"]);

    let (code, stdout, _) = repo.gg(&["pr", "--print"]);

    assert_eq!(code, 0);
    assert!(stdout.contains("github.com"));
}

#[test]
fn pr_github_ssh_url() {
    let repo = TempRepo::new();

    repo.run_git(&["remote", "add", "origin", "git@github.com:user/myrepo.git"]);

    let (code, stdout, _) = repo.gg(&["pr", "-p"]);

    assert_eq!(code, 0);
    assert!(stdout.contains("https://github.com/user/myrepo"));
    assert!(stdout.contains("compare"));
}

#[test]
fn pr_github_https_url() {
    let repo = TempRepo::new();

    repo.run_git(&["remote", "add", "origin", "https://github.com/user/myrepo.git"]);

    let (code, stdout, _) = repo.gg(&["pr", "-p"]);

    assert_eq!(code, 0);
    assert!(stdout.contains("https://github.com/user/myrepo"));
}

#[test]
fn pr_gitlab_url() {
    let repo = TempRepo::new();

    repo.run_git(&["remote", "add", "origin", "git@gitlab.com:user/myrepo.git"]);

    let (code, stdout, _) = repo.gg(&["pr", "-p"]);

    assert_eq!(code, 0);
    assert!(stdout.contains("gitlab.com"));
    assert!(stdout.contains("merge_requests"));
}

#[test]
fn pr_bitbucket_url() {
    let repo = TempRepo::new();

    repo.run_git(&["remote", "add", "origin", "git@bitbucket.org:user/myrepo.git"]);

    let (code, stdout, _) = repo.gg(&["pr", "-p"]);

    assert_eq!(code, 0);
    assert!(stdout.contains("bitbucket.org"));
    assert!(stdout.contains("pull-requests"));
}

#[test]
fn pr_includes_branch_name() {
    let repo = TempRepo::new();

    repo.run_git(&["remote", "add", "origin", "git@github.com:user/repo.git"]);
    repo.checkout_new_branch("feature-branch");

    let (code, stdout, _) = repo.gg(&["pr", "-p"]);

    assert_eq!(code, 0);
    assert!(stdout.contains("feature-branch"));
}

#[test]
fn pr_no_remote_fails_gracefully() {
    let repo = TempRepo::new();

    // No remote added
    let (code, _, stderr) = repo.gg(&["pr", "-p"]);

    assert_ne!(code, 0);
    assert!(!stderr.is_empty() || code != 0);
}
