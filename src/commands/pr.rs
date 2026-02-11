use clap::Args;

use crate::git;
use crate::utils::{get_branch_name, get_repo};

#[derive(Args)]
pub struct PrArgs {
    /// Just print the URL, don't open browser
    #[arg(short, long)]
    pub print: bool,
}

pub fn run(args: PrArgs) -> i32 {
    match run_inner(args) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("gg: {}", e);
            1
        }
    }
}

fn run_inner(args: PrArgs) -> Result<i32, Box<dyn std::error::Error>> {
    let repo = get_repo()?;
    let branch = get_branch_name(&repo).ok_or("Could not determine current branch")?;

    // Get the remote URL
    let remote_url = git::capture(&["remote", "get-url", "origin"])?;

    // Convert to web URL for PR creation
    let pr_url = build_pr_url(&remote_url, &branch)?;

    if args.print {
        println!("{}", pr_url);
        return Ok(0);
    }

    // Open in browser
    println!("Opening: {}", pr_url);
    open_url(&pr_url)
}

fn build_pr_url(remote_url: &str, branch: &str) -> Result<String, Box<dyn std::error::Error>> {
    // Handle various git URL formats
    let url = remote_url
        .trim()
        .trim_end_matches(".git")
        .replace("git@github.com:", "https://github.com/")
        .replace("git@gitlab.com:", "https://gitlab.com/")
        .replace("git@bitbucket.org:", "https://bitbucket.org/");

    // Determine the platform and construct PR URL
    if url.contains("github.com") {
        Ok(format!("{}/compare/{}?expand=1", url, branch))
    } else if url.contains("gitlab.com") {
        Ok(format!("{}/-/merge_requests/new?merge_request[source_branch]={}", url, branch))
    } else if url.contains("bitbucket.org") {
        Ok(format!("{}/pull-requests/new?source={}", url, branch))
    } else {
        // Generic fallback - just open the repo
        Ok(url)
    }
}

fn open_url(url: &str) -> Result<i32, Box<dyn std::error::Error>> {
    #[cfg(target_os = "macos")]
    let cmd = "open";
    #[cfg(target_os = "linux")]
    let cmd = "xdg-open";
    #[cfg(target_os = "windows")]
    let cmd = "start";

    std::process::Command::new(cmd)
        .arg(url)
        .spawn()
        .map_err(|e| format!("Failed to open browser: {}", e))?;

    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_pr_url_github_ssh() {
        let url = build_pr_url("git@github.com:user/repo.git", "feature-branch").unwrap();
        assert_eq!(url, "https://github.com/user/repo/compare/feature-branch?expand=1");
    }

    #[test]
    fn test_build_pr_url_github_https() {
        let url = build_pr_url("https://github.com/user/repo.git", "my-branch").unwrap();
        assert_eq!(url, "https://github.com/user/repo/compare/my-branch?expand=1");
    }

    #[test]
    fn test_build_pr_url_github_no_git_suffix() {
        let url = build_pr_url("https://github.com/user/repo", "branch").unwrap();
        assert_eq!(url, "https://github.com/user/repo/compare/branch?expand=1");
    }

    #[test]
    fn test_build_pr_url_gitlab_ssh() {
        let url = build_pr_url("git@gitlab.com:user/repo.git", "feature").unwrap();
        assert_eq!(url, "https://gitlab.com/user/repo/-/merge_requests/new?merge_request[source_branch]=feature");
    }

    #[test]
    fn test_build_pr_url_gitlab_https() {
        let url = build_pr_url("https://gitlab.com/user/repo.git", "branch").unwrap();
        assert_eq!(url, "https://gitlab.com/user/repo/-/merge_requests/new?merge_request[source_branch]=branch");
    }

    #[test]
    fn test_build_pr_url_bitbucket_ssh() {
        let url = build_pr_url("git@bitbucket.org:user/repo.git", "feature").unwrap();
        assert_eq!(url, "https://bitbucket.org/user/repo/pull-requests/new?source=feature");
    }

    #[test]
    fn test_build_pr_url_unknown_host() {
        let url = build_pr_url("https://git.company.com/user/repo.git", "branch").unwrap();
        // Should return the base URL as fallback
        assert_eq!(url, "https://git.company.com/user/repo");
    }

    #[test]
    fn test_build_pr_url_trims_whitespace() {
        let url = build_pr_url("  git@github.com:user/repo.git  \n", "branch").unwrap();
        assert_eq!(url, "https://github.com/user/repo/compare/branch?expand=1");
    }
}
