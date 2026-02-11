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
