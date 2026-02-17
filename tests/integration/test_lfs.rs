//! Integration tests for gg lfs commands

use crate::common::TempRepo;
use std::fs;

// ============================================
// LFS Help Tests
// ============================================

#[test]
fn lfs_help_shows_subcommands() {
    let repo = TempRepo::new();
    let (code, stdout, _) = repo.gg(&["lfs", "--help"]);

    assert_eq!(code, 0);
    assert!(stdout.contains("install") || stdout.contains("Install"));
    assert!(stdout.contains("track") || stdout.contains("Track"));
    assert!(stdout.contains("import") || stdout.contains("Import"));
    assert!(stdout.contains("migrate") || stdout.contains("Migrate"));
    assert!(stdout.contains("push") || stdout.contains("Push"));
    assert!(stdout.contains("pull") || stdout.contains("Pull"));
    assert!(stdout.contains("status") || stdout.contains("Status"));
}

#[test]
fn lfs_install_help() {
    let repo = TempRepo::new();
    let (code, stdout, _) = repo.gg(&["lfs", "install", "--help"]);

    assert_eq!(code, 0);
    assert!(stdout.contains("force") || stdout.contains("-f"));
}

#[test]
fn lfs_track_help() {
    let repo = TempRepo::new();
    let (code, stdout, _) = repo.gg(&["lfs", "track", "--help"]);

    assert_eq!(code, 0);
    assert!(stdout.contains("pattern") || stdout.contains("PATTERN"));
}

#[test]
fn lfs_push_help() {
    let repo = TempRepo::new();
    let (code, stdout, _) = repo.gg(&["lfs", "push", "--help"]);

    assert_eq!(code, 0);
    assert!(stdout.contains("dry-run") || stdout.contains("-n"));
    assert!(stdout.contains("all") || stdout.contains("-a"));
}

#[test]
fn lfs_pull_help() {
    let repo = TempRepo::new();
    let (code, stdout, _) = repo.gg(&["lfs", "pull", "--help"]);

    assert_eq!(code, 0);
    assert!(stdout.contains("dry-run") || stdout.contains("-n"));
}

#[test]
fn lfs_status_help() {
    let repo = TempRepo::new();
    let (code, stdout, _) = repo.gg(&["lfs", "status", "--help"]);

    assert_eq!(code, 0);
    assert!(stdout.contains("verbose") || stdout.contains("-v"));
}

// ============================================
// LFS Install Tests
// ============================================

#[test]
fn lfs_install_creates_hooks() {
    let repo = TempRepo::new();
    let (code, stdout, _) = repo.gg(&["lfs", "install"]);

    assert_eq!(code, 0);
    assert!(stdout.contains("Installed") || stdout.contains("installed"));

    // Check hooks exist
    let hooks_dir = repo.path.join(".git").join("hooks");
    assert!(hooks_dir.join("pre-push").exists());
    assert!(hooks_dir.join("post-checkout").exists());
    assert!(hooks_dir.join("post-merge").exists());
}

#[test]
fn lfs_install_creates_config() {
    let repo = TempRepo::new();
    repo.gg(&["lfs", "install"]);

    // Check config was created
    let config_path = repo.path.join(".gg").join("lfs.toml");
    assert!(config_path.exists());

    // Verify it's valid TOML with expected content
    let content = fs::read_to_string(&config_path).unwrap();
    assert!(content.contains("[storage]"));
    assert!(content.contains("bucket"));
}

#[test]
fn lfs_install_adds_to_gitignore() {
    let repo = TempRepo::new();
    repo.gg(&["lfs", "install"]);

    let gitignore = repo.path.join(".gitignore");
    let content = fs::read_to_string(&gitignore).unwrap();
    assert!(content.contains(".gg/") || content.contains(".gg"));
}

#[test]
fn lfs_install_force_overwrites_hooks() {
    let repo = TempRepo::new();

    // Create existing hook
    let hooks_dir = repo.path.join(".git").join("hooks");
    fs::create_dir_all(&hooks_dir).unwrap();
    let pre_push = hooks_dir.join("pre-push");
    fs::write(&pre_push, "#!/bin/sh\necho 'existing hook'\n").unwrap();

    // Install without force - should skip
    let (_, stdout, _) = repo.gg(&["lfs", "install"]);
    assert!(stdout.contains("Skipping") || stdout.contains("skip"));

    // Install with force - should overwrite
    let (_, stdout, _) = repo.gg(&["lfs", "install", "-f"]);
    assert!(stdout.contains("Installed") || stdout.contains("installed"));

    let hook_content = fs::read_to_string(&pre_push).unwrap();
    assert!(hook_content.contains("gg-lfs") || hook_content.contains("gg lfs"));
}

// ============================================
// LFS Uninstall Tests
// ============================================

#[test]
fn lfs_uninstall_removes_hooks() {
    let repo = TempRepo::new();

    // First install
    repo.gg(&["lfs", "install"]);

    // Then uninstall
    let (code, stdout, _) = repo.gg(&["lfs", "uninstall"]);
    assert_eq!(code, 0);
    assert!(stdout.contains("Removed") || stdout.contains("uninstall"));

    // Check hooks are removed
    let hooks_dir = repo.path.join(".git").join("hooks");
    assert!(!hooks_dir.join("pre-push").exists());
    assert!(!hooks_dir.join("post-checkout").exists());
    assert!(!hooks_dir.join("post-merge").exists());
}

#[test]
fn lfs_uninstall_preserves_non_lfs_hooks() {
    let repo = TempRepo::new();

    // Create a custom hook
    let hooks_dir = repo.path.join(".git").join("hooks");
    fs::create_dir_all(&hooks_dir).unwrap();
    let pre_commit = hooks_dir.join("pre-commit");
    fs::write(&pre_commit, "#!/bin/sh\necho 'custom hook'\n").unwrap();

    // Install LFS hooks
    repo.gg(&["lfs", "install"]);

    // Uninstall
    repo.gg(&["lfs", "uninstall"]);

    // Custom hook should still exist
    assert!(pre_commit.exists());
}

// ============================================
// LFS Track Tests
// ============================================

#[test]
fn lfs_track_creates_gitattributes() {
    let repo = TempRepo::new();

    let (code, _, _) = repo.gg(&["lfs", "track", "*.psd"]);
    assert_eq!(code, 0);

    let gitattributes = repo.path.join(".gitattributes");
    assert!(gitattributes.exists());

    let content = fs::read_to_string(&gitattributes).unwrap();
    assert!(content.contains("*.psd"));
    assert!(content.contains("filter=lfs"));
}

#[test]
fn lfs_track_appends_to_existing_gitattributes() {
    let repo = TempRepo::new();

    // Create existing .gitattributes
    let gitattributes = repo.path.join(".gitattributes");
    fs::write(&gitattributes, "*.txt text\n").unwrap();

    repo.gg(&["lfs", "track", "*.psd"]);

    let content = fs::read_to_string(&gitattributes).unwrap();
    assert!(content.contains("*.txt text"));
    assert!(content.contains("*.psd"));
    assert!(content.contains("filter=lfs"));
}

#[test]
fn lfs_track_multiple_patterns() {
    let repo = TempRepo::new();

    repo.gg(&["lfs", "track", "*.psd"]);
    repo.gg(&["lfs", "track", "*.zip"]);
    repo.gg(&["lfs", "track", "assets/**"]);

    let gitattributes = repo.path.join(".gitattributes");
    let content = fs::read_to_string(&gitattributes).unwrap();
    assert!(content.contains("*.psd"));
    assert!(content.contains("*.zip"));
    assert!(content.contains("assets/**"));
}

#[test]
fn lfs_track_stages_gitattributes() {
    let repo = TempRepo::new();

    repo.gg(&["lfs", "track", "*.psd"]);

    // Check it was staged
    let output = repo.run_git(&["status", "--porcelain"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(".gitattributes"));
}

// ============================================
// LFS Untrack Tests
// ============================================

#[test]
fn lfs_untrack_removes_pattern() {
    let repo = TempRepo::new();

    // Track then untrack
    repo.gg(&["lfs", "track", "*.psd"]);
    repo.gg(&["lfs", "untrack", "*.psd"]);

    let gitattributes = repo.path.join(".gitattributes");
    let content = fs::read_to_string(&gitattributes).unwrap();
    assert!(!content.contains("*.psd"));
}

#[test]
fn lfs_untrack_nonexistent_pattern_is_graceful() {
    let repo = TempRepo::new();

    let (code, stdout, _) = repo.gg(&["lfs", "untrack", "*.nonexistent"]);
    assert_eq!(code, 0);
    assert!(stdout.contains("not") || stdout.contains("was not being tracked"));
}

// ============================================
// LFS Status Tests
// ============================================

#[test]
fn lfs_status_shows_config() {
    let repo = TempRepo::new();
    repo.gg(&["lfs", "install"]);

    let (code, stdout, _) = repo.gg(&["lfs", "status"]);
    assert_eq!(code, 0);
    assert!(stdout.contains("Configuration") || stdout.contains("config"));
    assert!(stdout.contains("S3") || stdout.contains("s3") || stdout.contains("storage"));
}

#[test]
fn lfs_status_shows_no_config_message() {
    let repo = TempRepo::new();

    let (code, stdout, _) = repo.gg(&["lfs", "status"]);
    assert_eq!(code, 0);
    assert!(stdout.contains("Not configured") || stdout.contains("install"));
}

#[test]
fn lfs_status_shows_patterns() {
    let repo = TempRepo::new();

    repo.gg(&["lfs", "track", "*.psd"]);
    repo.gg(&["lfs", "track", "*.zip"]);

    let (_, stdout, _) = repo.gg(&["lfs", "status"]);
    assert!(stdout.contains("*.psd"));
    assert!(stdout.contains("*.zip"));
}

#[test]
fn lfs_status_shows_no_patterns_message() {
    let repo = TempRepo::new();

    let (_, stdout, _) = repo.gg(&["lfs", "status"]);
    assert!(stdout.contains("No patterns") || stdout.contains("track"));
}

#[test]
fn lfs_status_shows_hooks() {
    let repo = TempRepo::new();
    repo.gg(&["lfs", "install"]);

    let (_, stdout, _) = repo.gg(&["lfs", "status"]);
    assert!(stdout.contains("Hooks") || stdout.contains("hook"));
    assert!(stdout.contains("pre-push"));
    assert!(stdout.contains("installed"));
}

#[test]
fn lfs_status_verbose_flag() {
    let repo = TempRepo::new();

    repo.gg(&["lfs", "track", "*.psd"]);
    repo.create_file("test.psd", "fake psd content");

    let (code, stdout, _) = repo.gg(&["lfs", "status", "-v"]);
    assert_eq!(code, 0);
    // Verbose should show individual files or more details
    assert!(stdout.contains("test.psd") || stdout.contains("file") || stdout.contains("LFS"));
}

// ============================================
// LFS Push Tests
// ============================================

#[test]
fn lfs_push_no_config_shows_error() {
    let repo = TempRepo::new();

    repo.gg(&["lfs", "track", "*.psd"]);
    repo.create_file("test.psd", "fake psd content");

    let (code, stdout, stderr) = repo.gg(&["lfs", "push"]);
    assert_ne!(code, 0);

    let combined = format!("{}{}", stdout, stderr);
    assert!(combined.contains("install") || combined.contains("config") || combined.contains("Configuration"));
}

#[test]
fn lfs_push_no_patterns_shows_message() {
    let repo = TempRepo::new();
    repo.gg(&["lfs", "install"]);

    let (code, stdout, _) = repo.gg(&["lfs", "push"]);
    // Should indicate no patterns - either message or success with nothing to do
    assert!(stdout.contains("No LFS patterns") || stdout.contains("track") || code == 0);
}

#[test]
fn lfs_push_dry_run_flag() {
    let repo = TempRepo::new();
    repo.gg(&["lfs", "install"]);
    repo.gg(&["lfs", "track", "*.psd"]);
    repo.create_file("test.psd", "fake psd content");

    let (_, stdout, _) = repo.gg(&["lfs", "push", "-n"]);
    // Dry run should say what would happen or show no files
    assert!(stdout.contains("dry run") || stdout.contains("Would") || stdout.contains("No LFS"));
}

#[test]
fn lfs_push_all_flag() {
    let repo = TempRepo::new();
    repo.gg(&["lfs", "install"]);
    repo.gg(&["lfs", "track", "*.psd"]);
    repo.create_file("test.psd", "fake psd content");

    // The -a flag should be accepted
    let (_, _, stderr) = repo.gg(&["lfs", "push", "-n", "-a"]);
    // Should not fail due to flag parsing
    assert!(!stderr.contains("unexpected argument"));
}

// ============================================
// LFS Pull Tests
// ============================================

#[test]
fn lfs_pull_no_config_shows_message() {
    let repo = TempRepo::new();

    let (code, stdout, _) = repo.gg(&["lfs", "pull"]);
    // Should gracefully handle no config
    assert_eq!(code, 0);
    assert!(stdout.contains("No LFS") || stdout.contains("install") || stdout.contains("configuration"));
}

#[test]
fn lfs_pull_dry_run_flag() {
    let repo = TempRepo::new();
    repo.gg(&["lfs", "install"]);

    let (code, stdout, _) = repo.gg(&["lfs", "pull", "-n"]);
    // Should accept dry run flag
    assert!(code == 0 || stdout.contains("dry") || stdout.contains("No LFS"));
}

#[test]
fn lfs_pull_include_flag() {
    let repo = TempRepo::new();
    repo.gg(&["lfs", "install"]);

    // Should accept include flag
    let (_, _, stderr) = repo.gg(&["lfs", "pull", "--include", "*.psd"]);
    assert!(!stderr.contains("unexpected argument"));
}

#[test]
fn lfs_pull_exclude_flag() {
    let repo = TempRepo::new();
    repo.gg(&["lfs", "install"]);

    // Should accept exclude flag
    let (_, _, stderr) = repo.gg(&["lfs", "pull", "--exclude", "*.zip"]);
    assert!(!stderr.contains("unexpected argument"));
}

// ============================================
// LFS Verify Tests
// ============================================

#[test]
fn lfs_verify_help() {
    let repo = TempRepo::new();
    let (code, stdout, _) = repo.gg(&["lfs", "verify", "--help"]);

    assert_eq!(code, 0);
    assert!(stdout.contains("write") || stdout.contains("-w"));
}

#[test]
fn lfs_verify_no_config_shows_error() {
    let repo = TempRepo::new();

    let (code, stdout, stderr) = repo.gg(&["lfs", "verify"]);
    assert_ne!(code, 0);

    let combined = format!("{}{}", stdout, stderr);
    assert!(combined.contains("Configuration") || combined.contains("config") || combined.contains("install"));
}

#[test]
fn lfs_verify_with_config_checks_bucket() {
    let repo = TempRepo::new();
    repo.gg(&["lfs", "install"]);

    // The verify command should run but fail on bucket access (no real AWS)
    let (code, stdout, stderr) = repo.gg(&["lfs", "verify"]);

    // Should show it's checking things
    let combined = format!("{}{}", stdout, stderr);
    assert!(combined.contains("Checking") || combined.contains("Verifying") || combined.contains("Configuration"));

    // Should fail on bucket access (no real AWS credentials)
    // This is expected in a test environment
    assert!(code != 0 || combined.contains("FAILED") || combined.contains("Error"));
}

#[test]
fn lfs_verify_write_flag_accepted() {
    let repo = TempRepo::new();
    repo.gg(&["lfs", "install"]);

    // The --write flag should be accepted
    let (_, _, stderr) = repo.gg(&["lfs", "verify", "--write"]);
    assert!(!stderr.contains("unexpected argument"));
}

// ============================================
// LFS Import Tests
// ============================================

#[test]
fn lfs_import_help() {
    let repo = TempRepo::new();
    let (code, stdout, _) = repo.gg(&["lfs", "import", "--help"]);

    assert_eq!(code, 0);
    assert!(stdout.contains("dry-run") || stdout.contains("-n"));
    assert!(stdout.contains("include") || stdout.contains("-i"));
    assert!(stdout.contains("exclude") || stdout.contains("-e"));
}

#[test]
fn lfs_import_no_config_shows_error() {
    let repo = TempRepo::new();

    repo.gg(&["lfs", "track", "*.psd"]);
    repo.create_file("test.psd", "fake psd content");

    let (code, stdout, stderr) = repo.gg(&["lfs", "import"]);
    assert_ne!(code, 0);

    let combined = format!("{}{}", stdout, stderr);
    assert!(combined.contains("install") || combined.contains("config") || combined.contains("Configuration"));
}

#[test]
fn lfs_import_no_patterns_shows_message() {
    let repo = TempRepo::new();
    repo.gg(&["lfs", "install"]);

    let (code, stdout, _) = repo.gg(&["lfs", "import"]);
    assert!(stdout.contains("No LFS patterns") || stdout.contains("track") || code == 0);
}

#[test]
fn lfs_import_dry_run() {
    let repo = TempRepo::new();
    repo.gg(&["lfs", "install"]);
    repo.gg(&["lfs", "track", "*.psd"]);
    repo.create_file("test.psd", "fake psd content");

    let (_, stdout, _) = repo.gg(&["lfs", "import", "-n"]);
    assert!(stdout.contains("dry run") || stdout.contains("Would") || stdout.contains("Dry run") || stdout.contains("No files"));
}

#[test]
fn lfs_import_include_exclude_flags() {
    let repo = TempRepo::new();
    repo.gg(&["lfs", "install"]);
    repo.gg(&["lfs", "track", "*.psd"]);

    // Should accept include and exclude flags without parse error
    let (_, _, stderr) = repo.gg(&["lfs", "import", "--include", "*.psd", "--exclude", "*.zip"]);
    assert!(!stderr.contains("unexpected argument"));
}

// ============================================
// LFS Migrate Tests (git-lfs -> gg lfs)
// ============================================

#[test]
fn lfs_migrate_help() {
    let repo = TempRepo::new();
    let (code, stdout, _) = repo.gg(&["lfs", "migrate", "--help"]);

    assert_eq!(code, 0);
    assert!(stdout.contains("dry-run") || stdout.contains("-n"));
    assert!(stdout.contains("skip-fetch"));
    assert!(stdout.contains("keep-gitlfs"));
}

#[test]
fn lfs_migrate_no_config_shows_error() {
    let repo = TempRepo::new();
    repo.gg(&["lfs", "track", "*.psd"]);

    let (code, stdout, stderr) = repo.gg(&["lfs", "migrate"]);
    assert_ne!(code, 0);

    let combined = format!("{}{}", stdout, stderr);
    assert!(combined.contains("install") || combined.contains("config") || combined.contains("Configuration") || combined.contains("git-lfs"));
}

#[test]
fn lfs_migrate_no_patterns_shows_error() {
    let repo = TempRepo::new();
    repo.gg(&["lfs", "install"]);

    let (code, stdout, stderr) = repo.gg(&["lfs", "migrate", "--skip-fetch"]);
    let combined = format!("{}{}", stdout, stderr);
    // Should indicate no LFS patterns found
    assert!(combined.contains("No LFS patterns") || combined.contains("gitattributes") || combined.contains("git-lfs") || code != 0);
}

#[test]
fn lfs_migrate_dry_run() {
    let repo = TempRepo::new();
    repo.gg(&["lfs", "install"]);
    repo.gg(&["lfs", "track", "*.psd"]);
    repo.create_file("test.psd", "fake psd content");

    let (_, stdout, _) = repo.gg(&["lfs", "migrate", "-n", "--skip-fetch"]);
    assert!(stdout.contains("dry run") || stdout.contains("Would") || stdout.contains("Dry run") || stdout.contains("No LFS files"));
}

#[test]
fn lfs_migrate_skip_fetch_flag() {
    let repo = TempRepo::new();
    repo.gg(&["lfs", "install"]);

    // Should accept --skip-fetch without parse error
    let (_, _, stderr) = repo.gg(&["lfs", "migrate", "-n", "--skip-fetch"]);
    assert!(!stderr.contains("unexpected argument"));
}

#[test]
fn lfs_migrate_keep_gitlfs_flag() {
    let repo = TempRepo::new();
    repo.gg(&["lfs", "install"]);

    // Should accept --keep-gitlfs without parse error
    let (_, _, stderr) = repo.gg(&["lfs", "migrate", "-n", "--skip-fetch", "--keep-gitlfs"]);
    assert!(!stderr.contains("unexpected argument"));
}

// ============================================
// LFS Clean Filter Tests
// ============================================

/// Helper: run `gg lfs clean` with piped stdin in a given directory
fn run_gg_clean(dir: &std::path::Path, stdin_data: &[u8]) -> (i32, Vec<u8>, String) {
    use std::io::Write;
    use std::process::{Command, Stdio};

    let mut child = Command::new(env!("CARGO_BIN_EXE_gg"))
        .args(["lfs", "clean", "test.bin"])
        .current_dir(dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn gg lfs clean");

    child
        .stdin
        .take()
        .unwrap()
        .write_all(stdin_data)
        .unwrap();
    let output = child.wait_with_output().unwrap();

    let code = output.status.code().unwrap_or(-1);
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (code, output.stdout, stderr)
}

/// Helper: run `gg lfs smudge` with piped stdin in a given directory
fn run_gg_smudge(dir: &std::path::Path, stdin_data: &[u8]) -> (i32, Vec<u8>, String) {
    use std::io::Write;
    use std::process::{Command, Stdio};

    let mut child = Command::new(env!("CARGO_BIN_EXE_gg"))
        .args(["lfs", "smudge", "test.bin"])
        .current_dir(dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn gg lfs smudge");

    child
        .stdin
        .take()
        .unwrap()
        .write_all(stdin_data)
        .unwrap();
    let output = child.wait_with_output().unwrap();

    let code = output.status.code().unwrap_or(-1);
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (code, output.stdout, stderr)
}

#[test]
fn lfs_clean_produces_pointer() {
    let repo = TempRepo::new();
    let content = b"This is some binary content for LFS testing.\x00\x01\x02\x03";

    let (code, stdout, _) = run_gg_clean(&repo.path, content);
    assert_eq!(code, 0);

    let output = String::from_utf8_lossy(&stdout);
    assert!(
        output.contains("version https://git-lfs.github.com/spec/v1"),
        "Expected LFS pointer version line, got: {}",
        output
    );
    assert!(output.contains("oid sha256:"));
    assert!(output.contains(&format!("size {}", content.len())));
}

#[test]
fn lfs_clean_passthrough_pointer() {
    let repo = TempRepo::new();

    // Create a valid pointer
    let pointer = "version https://git-lfs.github.com/spec/v1\noid sha256:4d7a214614ab2935c943f9e0ff69d22eadbb8f32b1258daaa5e2ca24d17e2393\nsize 12345\n";

    let (code, stdout, _) = run_gg_clean(&repo.path, pointer.as_bytes());
    assert_eq!(code, 0);

    let output = String::from_utf8_lossy(&stdout);
    assert_eq!(output.as_ref(), pointer, "Pointer should pass through unchanged");
}

#[test]
fn lfs_smudge_passthrough_non_pointer() {
    let repo = TempRepo::new();
    let content = b"This is not a pointer file at all.";

    let (code, stdout, _) = run_gg_smudge(&repo.path, content);
    assert_eq!(code, 0);
    assert_eq!(&stdout, content, "Non-pointer content should pass through unchanged");
}

#[test]
fn lfs_smudge_from_cache() {
    let repo = TempRepo::new();
    let content = b"Binary content that gets cleaned then smudged.\x00\xFF\xFE";

    // Step 1: Clean the content (populates cache)
    let (code, pointer_out, _) = run_gg_clean(&repo.path, content);
    assert_eq!(code, 0);

    // Step 2: Smudge the pointer (should restore from cache)
    let (code, restored, _) = run_gg_smudge(&repo.path, &pointer_out);
    assert_eq!(code, 0);
    assert_eq!(
        &restored, content,
        "Smudge should restore original content from cache"
    );
}

// ============================================
// LFS Install/Uninstall Filter Driver Tests
// ============================================

#[test]
fn lfs_install_registers_filter_driver() {
    let repo = TempRepo::new();
    repo.gg(&["lfs", "install"]);

    let clean = repo.git_output(&["config", "filter.lfs.clean"]);
    let smudge = repo.git_output(&["config", "filter.lfs.smudge"]);
    let required = repo.git_output(&["config", "filter.lfs.required"]);

    assert_eq!(clean, "gg lfs clean %f");
    assert_eq!(smudge, "gg lfs smudge %f");
    assert_eq!(required, "true");
}

#[test]
fn lfs_uninstall_removes_filter_driver() {
    let repo = TempRepo::new();

    // Install first
    repo.gg(&["lfs", "install"]);

    // Verify it's set locally
    let clean = repo.git_output(&["config", "--local", "filter.lfs.clean"]);
    assert_eq!(clean, "gg lfs clean %f");

    // Uninstall
    repo.gg(&["lfs", "uninstall"]);

    // Verify the local config no longer has it
    let output = repo.run_git(&["config", "--local", "filter.lfs.clean"]);
    assert!(
        !output.status.success(),
        "filter.lfs.clean should be unset in local config after uninstall"
    );
}

// ============================================
// LFS Filter Git Integration Test
// ============================================

#[test]
fn lfs_filter_roundtrip_via_git() {
    let repo = TempRepo::new();

    // Install (registers hooks + filter driver)
    let (code, _, _) = repo.gg(&["lfs", "install"]);
    assert_eq!(code, 0);

    // Track *.bin files
    let (code, _, _) = repo.gg(&["lfs", "track", "*.bin"]);
    assert_eq!(code, 0);

    // Create a binary file
    let content = b"Large binary content for roundtrip test\x00\x01\x02";
    fs::write(repo.path.join("test.bin"), content).unwrap();

    // Stage the file — this invokes the clean filter
    repo.run_git(&["add", "test.bin"]);
    repo.run_git(&["add", ".gitattributes"]);

    // Check what's in the git index (should be a pointer)
    let index_content = repo.git_output(&["show", ":test.bin"]);
    assert!(
        index_content.contains("version https://git-lfs.github.com/spec/v1"),
        "Index should contain pointer, got: {}",
        index_content
    );
    assert!(index_content.contains("oid sha256:"));
    assert!(index_content.contains(&format!("size {}", content.len())));

    // Working tree should still have the real content
    let working_content = fs::read(repo.path.join("test.bin")).unwrap();
    assert_eq!(
        &working_content, content,
        "Working tree should still have real content"
    );
}

// ============================================
// LFS Clean Filter Edge Case Tests
// ============================================

#[test]
fn lfs_clean_empty_input() {
    let repo = TempRepo::new();
    let (code, stdout, _) = run_gg_clean(&repo.path, b"");
    assert_eq!(code, 0);
    // Empty input should produce a pointer (zero-size)
    let output = String::from_utf8_lossy(&stdout);
    assert!(output.contains("version https://git-lfs.github.com/spec/v1"));
    assert!(output.contains("size 0"));
}

#[test]
fn lfs_clean_deterministic_hash() {
    let repo = TempRepo::new();
    let content = b"deterministic content";

    let (_, stdout1, _) = run_gg_clean(&repo.path, content);
    let (_, stdout2, _) = run_gg_clean(&repo.path, content);

    assert_eq!(stdout1, stdout2, "Same content should produce identical pointers");
}

#[test]
fn lfs_clean_different_content_different_hash() {
    let repo = TempRepo::new();

    let (_, stdout1, _) = run_gg_clean(&repo.path, b"content A");
    let (_, stdout2, _) = run_gg_clean(&repo.path, b"content B");

    assert_ne!(stdout1, stdout2, "Different content should produce different pointers");
}

// ============================================
// LFS Smudge Filter Edge Case Tests
// ============================================

#[test]
fn lfs_smudge_empty_input() {
    let repo = TempRepo::new();
    let (code, stdout, _) = run_gg_smudge(&repo.path, b"");
    assert_eq!(code, 0);
    // Empty input is not a pointer, should pass through as-is
    assert!(stdout.is_empty());
}

#[test]
fn lfs_smudge_corrupted_pointer() {
    let repo = TempRepo::new();
    // Looks like a pointer but has invalid fields
    let corrupted = b"version https://git-lfs.github.com/spec/v1\noid sha256:invalid\nsize abc\n";
    let (code, stdout, _) = run_gg_smudge(&repo.path, corrupted);
    assert_eq!(code, 0);
    // Should pass through unchanged (parse failure = not a pointer)
    assert_eq!(&stdout, corrupted);
}

#[test]
fn lfs_smudge_partial_pointer() {
    let repo = TempRepo::new();
    // Missing size field
    let partial = b"version https://git-lfs.github.com/spec/v1\noid sha256:4d7a214614ab2935c943f9e0ff69d22eadbb8f32b1258daaa5e2ca24d17e2393\n";
    let (code, stdout, _) = run_gg_smudge(&repo.path, partial);
    assert_eq!(code, 0);
    // Should pass through unchanged
    assert_eq!(&stdout, partial);
}

#[test]
fn lfs_smudge_cache_miss_no_config() {
    let repo = TempRepo::new();
    // Valid pointer but no LFS config and no cache entry
    let pointer = b"version https://git-lfs.github.com/spec/v1\noid sha256:4d7a214614ab2935c943f9e0ff69d22eadbb8f32b1258daaa5e2ca24d17e2393\nsize 12345\n";
    let (code, stdout, stderr) = run_gg_smudge(&repo.path, pointer);
    assert_eq!(code, 0);
    // Should gracefully degrade: output pointer content + warning on stderr
    let output = String::from_utf8_lossy(&stdout);
    assert!(output.contains("version https://git-lfs.github.com/spec/v1"));
    let stderr_str = String::from_utf8_lossy(stderr.as_bytes());
    assert!(
        stderr_str.contains("warning") || output.contains("version"),
        "Should warn or pass through pointer on cache miss"
    );
}

// ============================================
// LFS Install Idempotency Tests
// ============================================

#[test]
fn lfs_install_idempotent_runs_twice() {
    let repo = TempRepo::new();

    // Install once
    let (code1, _, _) = repo.gg(&["lfs", "install"]);
    assert_eq!(code1, 0);

    // Install again — hooks are ours, should overwrite without error
    let (code2, stdout2, _) = repo.gg(&["lfs", "install"]);
    assert_eq!(code2, 0);
    assert!(
        stdout2.contains("Installed") || stdout2.contains("installed"),
        "Second install should succeed and re-install our hooks"
    );

    // Hooks should still work
    let hooks_dir = repo.path.join(".git").join("hooks");
    let content = fs::read_to_string(hooks_dir.join("pre-push")).unwrap();
    assert!(content.contains("gg-lfs"));
}

#[test]
fn lfs_install_does_not_duplicate_gitignore() {
    let repo = TempRepo::new();

    repo.gg(&["lfs", "install"]);
    repo.gg(&["lfs", "install"]);

    let content = fs::read_to_string(repo.path.join(".gitignore")).unwrap();
    let count = content.matches(".gg/").count();
    assert_eq!(count, 1, ".gg/ should appear only once in .gitignore");
}

// ============================================
// LFS Filter Full Roundtrip Test
// ============================================

#[test]
fn lfs_filter_checkout_roundtrip() {
    let repo = TempRepo::new();

    // Install and track
    repo.gg(&["lfs", "install"]);
    repo.gg(&["lfs", "track", "*.bin"]);

    // Create binary file, stage and commit
    let content = b"Binary content for checkout roundtrip\x00\xFF";
    fs::write(repo.path.join("test.bin"), content).unwrap();
    repo.run_git(&["add", ".gitattributes", "test.bin"]);
    repo.run_git(&["commit", "-m", "add binary file"]);

    // Verify index has pointer
    let index_content = repo.git_output(&["show", "HEAD:test.bin"]);
    assert!(index_content.contains("version https://git-lfs.github.com/spec/v1"));

    // Remove working copy and checkout again (triggers smudge)
    fs::remove_file(repo.path.join("test.bin")).unwrap();
    repo.run_git(&["checkout", "--", "test.bin"]);

    // Working tree should have real content restored from cache
    let restored = fs::read(repo.path.join("test.bin")).unwrap();
    assert_eq!(
        &restored, content,
        "Checkout should restore original content via smudge filter"
    );
}

// ============================================
// CLI Tests
// ============================================

fn gg() -> std::process::Command {
    std::process::Command::new(env!("CARGO_BIN_EXE_gg"))
}

#[test]
fn cli_lfs_in_main_help() {
    let output = gg().arg("--help").output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("lfs") || stdout.contains("Lfs") || stdout.contains("LFS"));
}

#[test]
fn cli_lfs_unknown_subcommand() {
    let output = gg()
        .args(["lfs", "unknown-command"])
        .output()
        .unwrap();
    assert!(!output.status.success());
}
