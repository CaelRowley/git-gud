use git2::{BranchType, Repository};

/// Open the repository from the current directory (or any parent).
pub fn get_repo() -> Result<Repository, git2::Error> {
    Repository::discover(".")
}

/// Get the current branch name.
pub fn get_branch_name(repo: &Repository) -> Option<String> {
    repo.head().ok()?.shorthand().map(String::from)
}

/// Check if a branch name is the main/master branch.
pub fn is_main_branch(branch: &str) -> bool {
    matches!(branch, "main" | "master")
}

/// Get the name of the main branch (prefers "main" over "master").
pub fn get_main_branch_name(repo: &Repository) -> &'static str {
    if repo.find_branch("main", BranchType::Local).is_ok() {
        "main"
    } else {
        "master"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_main_branch_main() {
        assert!(is_main_branch("main"));
    }

    #[test]
    fn test_is_main_branch_master() {
        assert!(is_main_branch("master"));
    }

    #[test]
    fn test_is_main_branch_feature() {
        assert!(!is_main_branch("feature/foo"));
    }

    #[test]
    fn test_is_main_branch_develop() {
        assert!(!is_main_branch("develop"));
    }

    #[test]
    fn test_get_repo_in_git_dir() {
        // This test runs from within the git-gud repo itself
        let result = get_repo();
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_branch_name() {
        let repo = get_repo().expect("Should be in a git repo");
        let branch = get_branch_name(&repo);
        assert!(branch.is_some());
    }
}
