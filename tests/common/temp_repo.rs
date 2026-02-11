use std::path::PathBuf;
use std::process::{Command, Output};
use tempfile::TempDir;

/// A temporary git repository for testing.
/// Automatically cleaned up when dropped.
pub struct TempRepo {
    #[allow(dead_code)]
    pub dir: TempDir,
    pub path: PathBuf,
}

impl TempRepo {
    /// Create a new temporary git repository with initial commit.
    pub fn new() -> Self {
        let dir = TempDir::new().expect("Failed to create temp directory");
        let path = dir.path().to_path_buf();

        let repo = Self { dir, path };

        // Initialize git repo with main as default branch
        repo.run_git(&["init", "-b", "main"]);
        repo.run_git(&["config", "user.email", "test@example.com"]);
        repo.run_git(&["config", "user.name", "Test User"]);

        // Create initial commit so we have a valid HEAD
        repo.create_file("README.md", "# Test Repo\n");
        repo.run_git(&["add", "."]);
        repo.run_git(&["commit", "-m", "Initial commit"]);

        repo
    }

    /// Create a new temporary git repository with a fake remote.
    pub fn with_remote() -> Self {
        let repo = Self::new();

        // Create a bare repo to act as remote
        let remote_dir = TempDir::new().expect("Failed to create remote temp directory");
        let remote_path = remote_dir.path();

        Command::new("git")
            .args(["init", "--bare"])
            .current_dir(remote_path)
            .output()
            .expect("Failed to init bare repo");

        // Add as remote and push
        let remote_url = format!("file://{}", remote_path.display());
        repo.run_git(&["remote", "add", "origin", &remote_url]);
        repo.run_git(&["push", "-u", "origin", "main"]);

        // Keep the remote alive by leaking it (it will be cleaned up when process exits)
        std::mem::forget(remote_dir);

        repo
    }

    /// Run a git command in the repository.
    pub fn run_git(&self, args: &[&str]) -> Output {
        Command::new("git")
            .args(args)
            .current_dir(&self.path)
            .output()
            .expect("Failed to run git command")
    }

    /// Run a git command and return stdout as string.
    pub fn git_output(&self, args: &[&str]) -> String {
        let output = self.run_git(args);
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    }

    /// Run the gg binary in this repository.
    pub fn run_gg(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_gg"))
            .args(args)
            .current_dir(&self.path)
            .output()
            .expect("Failed to run gg command")
    }

    /// Run gg and return (exit_code, stdout, stderr).
    pub fn gg(&self, args: &[&str]) -> (i32, String, String) {
        let output = self.run_gg(args);
        let code = output.status.code().unwrap_or(-1);
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        (code, stdout, stderr)
    }

    /// Create a file in the repository.
    pub fn create_file(&self, name: &str, content: &str) {
        let file_path = self.path.join(name);
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent).expect("Failed to create parent directories");
        }
        std::fs::write(&file_path, content).expect("Failed to write file");
    }

    /// Modify an existing file.
    pub fn modify_file(&self, name: &str, content: &str) {
        self.create_file(name, content);
    }

    /// Delete a file from the repository.
    pub fn delete_file(&self, name: &str) {
        let file_path = self.path.join(name);
        std::fs::remove_file(&file_path).expect("Failed to delete file");
    }

    /// Create a commit with the given message.
    pub fn commit(&self, msg: &str) {
        self.run_git(&["add", "."]);
        self.run_git(&["commit", "-m", msg, "--allow-empty"]);
    }

    /// Create a new branch.
    #[allow(dead_code)]
    pub fn create_branch(&self, name: &str) {
        self.run_git(&["branch", name]);
    }

    /// Checkout a branch.
    pub fn checkout(&self, branch: &str) {
        self.run_git(&["checkout", branch]);
    }

    /// Create and checkout a new branch.
    pub fn checkout_new_branch(&self, name: &str) {
        self.run_git(&["checkout", "-b", name]);
    }

    /// Get the current branch name.
    pub fn current_branch(&self) -> String {
        self.git_output(&["rev-parse", "--abbrev-ref", "HEAD"])
    }

    /// Get the number of commits.
    pub fn commit_count(&self) -> usize {
        let output = self.git_output(&["rev-list", "--count", "HEAD"]);
        output.parse().unwrap_or(0)
    }

    /// Get the last commit message.
    pub fn last_commit_message(&self) -> String {
        self.git_output(&["log", "-1", "--format=%s"])
    }

    /// Check if there are staged changes.
    pub fn has_staged_changes(&self) -> bool {
        let output = self.run_git(&["diff", "--cached", "--quiet"]);
        !output.status.success()
    }

    /// Check if there are unstaged changes.
    pub fn has_unstaged_changes(&self) -> bool {
        let output = self.run_git(&["diff", "--quiet"]);
        !output.status.success()
    }

    /// Check if there are untracked files.
    pub fn has_untracked_files(&self) -> bool {
        let output = self.git_output(&["ls-files", "--others", "--exclude-standard"]);
        !output.is_empty()
    }

    /// Get list of branches.
    pub fn branches(&self) -> Vec<String> {
        self.git_output(&["branch", "--format=%(refname:short)"])
            .lines()
            .map(String::from)
            .collect()
    }

    /// Stage a file.
    pub fn stage(&self, file: &str) {
        self.run_git(&["add", file]);
    }

    /// Stage all changes.
    pub fn stage_all(&self) {
        self.run_git(&["add", "-A"]);
    }
}

impl Default for TempRepo {
    fn default() -> Self {
        Self::new()
    }
}
