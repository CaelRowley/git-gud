---
name: Git-gud Test Suite
overview: Comprehensive test suite with unit tests (mocked git), integration tests (temp repos), and CLI tests for all 12 commands and git passthrough.
todos:
  - id: add-dev-deps
    content: Add assert_cmd, predicates, tempfile to Cargo.toml dev-dependencies
    status: completed
  - id: temp-repo-helper
    content: Create tests/common/temp_repo.rs with TempRepo struct and helpers
    status: completed
  - id: cli-tests
    content: Create tests/cli/test_cli.rs with assert_cmd tests for help, version, aliases
    status: completed
  - id: integration-status
    content: Create integration tests for gg status (all states, -s flag)
    status: completed
  - id: integration-push
    content: Create integration tests for gg push (main, branch, -f flag)
    status: completed
  - id: integration-sync
    content: Create integration tests for gg sync (main, branch, --no-stash)
    status: completed
  - id: integration-qc
    content: Create integration tests for gg quick-commit (-A, -p flags)
    status: completed
  - id: integration-amend-undo
    content: Create integration tests for gg amend and gg undo
    status: completed
  - id: integration-pr
    content: Create integration tests for gg pr (-p flag, error cases)
    status: completed
  - id: integration-branches
    content: Create integration tests for clean-branches, recent, sw
    status: completed
  - id: integration-log-cmds
    content: Create integration tests for today and standup
    status: completed
  - id: integration-passthrough
    content: Create integration tests for git passthrough (flags, exit codes)
    status: completed
isProject: false
---

# Git-gud Comprehensive Test Plan

## Testing Architecture

```
tests/
├── common/
│   ├── mod.rs           # Shared test utilities
│   └── temp_repo.rs     # TempRepo helper for creating test git repos
├── integration/
│   ├── mod.rs
│   ├── test_status.rs
│   ├── test_push.rs
│   ├── test_sync.rs
│   ├── test_quick_commit.rs
│   ├── test_amend.rs
│   ├── test_undo.rs
│   ├── test_pr.rs
│   ├── test_clean_branches.rs
│   ├── test_recent.rs
│   ├── test_sw.rs
│   ├── test_today.rs
│   ├── test_standup.rs
│   └── test_passthrough.rs
└── cli/
    ├── mod.rs
    └── test_cli.rs      # End-to-end CLI tests with assert_cmd
```

## Dependencies to Add

```toml
[dev-dependencies]
assert_cmd = "2"
predicates = "3"
tempfile = "3"
```

---

## Test Categories

### 1. Unit Tests (in-module, already started)

Pure function tests that don't need git:

- `config.rs` - Theme defaults, NO_COLOR detection
- `utils/repo.rs` - `is_main_branch()` logic
- `commands/pr.rs` - URL building for GitHub/GitLab/Bitbucket
- `commands/standup.rs` - Workday calculation logic

### 2. Integration Tests (temp repos)

Each command tested in a real temporary git repository.

### 3. CLI Tests (binary execution)

Test the `gg` binary directly with `assert_cmd`.

---

## Test Matrix by Command

### `gg status` / `gg s`


| Test Case                | Type        | Description                      |
| ------------------------ | ----------- | -------------------------------- |
| `status_empty_repo`      | Integration | New repo with no commits         |
| `status_clean`           | Integration | No changes to report             |
| `status_staged_files`    | Integration | Files in index                   |
| `status_unstaged_files`  | Integration | Modified tracked files           |
| `status_untracked_files` | Integration | New untracked files              |
| `status_deleted_files`   | Integration | Deleted files                    |
| `status_mixed`           | Integration | Combination of all states        |
| `status_short_flag`      | Integration | `-s` delegates to git            |
| `status_alias_s`         | CLI         | `gg s` works same as `gg status` |


### `gg push` / `gg p`


| Test Case                      | Type        | Description                    |
| ------------------------------ | ----------- | ------------------------------ |
| `push_on_main`                 | Integration | Simple push on main branch     |
| `push_on_branch_no_upstream`   | Integration | Auto sets upstream             |
| `push_on_branch_with_upstream` | Integration | Normal push                    |
| `push_force_flag`              | Integration | `-f` uses `--force-with-lease` |
| `push_alias_p`                 | CLI         | `gg p` works                   |


### `gg sync`


| Test Case                       | Type        | Description                  |
| ------------------------------- | ----------- | ---------------------------- |
| `sync_on_main`                  | Integration | Pull rebase on main          |
| `sync_on_branch`                | Integration | Full rebase workflow         |
| `sync_with_uncommitted_changes` | Integration | Stash/unstash works          |
| `sync_no_stash_flag`            | Integration | `--no-stash` skips stash     |
| `sync_rebase_conflict`          | Integration | Handles conflicts gracefully |


### `gg quick-commit` / `gg qc`


| Test Case         | Type        | Description              |
| ----------------- | ----------- | ------------------------ |
| `qc_basic`        | Integration | Commits tracked changes  |
| `qc_all_flag`     | Integration | `-A` includes untracked  |
| `qc_push_flag`    | Integration | `-p` pushes after commit |
| `qc_all_and_push` | Integration | `-Ap` combined           |
| `qc_no_changes`   | Integration | Fails gracefully         |
| `qc_alias`        | CLI         | `gg qc` works            |


### `gg amend`


| Test Case          | Type        | Description                 |
| ------------------ | ----------- | --------------------------- |
| `amend_basic`      | Integration | Amends without message edit |
| `amend_all_flag`   | Integration | `-a` stages all first       |
| `amend_edit_flag`  | Integration | `-e` opens editor (mock)    |
| `amend_no_commits` | Integration | Fails on empty repo         |


### `gg undo`


| Test Case         | Type        | Description                 |
| ----------------- | ----------- | --------------------------- |
| `undo_default`    | Integration | Undoes 1 commit, soft reset |
| `undo_count`      | Integration | `gg undo 3` undoes 3        |
| `undo_hard_flag`  | Integration | `--hard` discards changes   |
| `undo_no_commits` | Integration | Fails gracefully            |


### `gg pr`


| Test Case         | Type        | Description                    |
| ----------------- | ----------- | ------------------------------ |
| `pr_github_ssh`   | Unit        | Correct URL from SSH remote    |
| `pr_github_https` | Unit        | Correct URL from HTTPS         |
| `pr_gitlab`       | Unit        | GitLab MR URL format           |
| `pr_bitbucket`    | Unit        | Bitbucket PR URL format        |
| `pr_print_flag`   | Integration | `-p` prints instead of opening |
| `pr_no_remote`    | Integration | Fails gracefully               |


### `gg clean-branches`


| Test Case             | Type        | Description                      |
| --------------------- | ----------- | -------------------------------- |
| `clean_dry_run`       | Integration | Default shows but doesn't delete |
| `clean_force_flag`    | Integration | `-f` actually deletes            |
| `clean_no_merged`     | Integration | No branches to clean             |
| `clean_protects_main` | Integration | Never deletes main/master        |


### `gg recent`


| Test Case           | Type        | Description             |
| ------------------- | ----------- | ----------------------- |
| `recent_default`    | Integration | Shows up to 10 branches |
| `recent_count_flag` | Integration | `-c 5` limits to 5      |
| `recent_no_history` | Integration | Handles empty reflog    |


### `gg sw`


| Test Case           | Type        | Description                         |
| ------------------- | ----------- | ----------------------------------- |
| `sw_with_number`    | Integration | `gg sw 2` switches directly         |
| `sw_invalid_number` | Integration | Fails gracefully                    |
| `sw_interactive`    | Integration | Prompts when no number (mock stdin) |


### `gg today`


| Test Case          | Type        | Description               |
| ------------------ | ----------- | ------------------------- |
| `today_default`    | Integration | Shows only user's commits |
| `today_all_flag`   | Integration | `-a` shows all authors    |
| `today_no_commits` | Integration | Empty output, no error    |


### `gg standup`


| Test Case           | Type        | Description               |
| ------------------- | ----------- | ------------------------- |
| `standup_default`   | Integration | Auto-detects last workday |
| `standup_all_flag`  | Integration | `-a` shows all authors    |
| `standup_days_flag` | Integration | `-d 5` looks back 5 days  |
| `standup_monday`    | Unit        | Returns 3 days (Friday)   |
| `standup_sunday`    | Unit        | Returns 2 days (Friday)   |
| `standup_weekday`   | Unit        | Returns 1 day (yesterday) |


### Git Passthrough


| Test Case                         | Type        | Description               |
| --------------------------------- | ----------- | ------------------------- |
| `passthrough_simple`              | Integration | `gg log` works            |
| `passthrough_with_flags`          | Integration | `gg log --oneline -5`     |
| `passthrough_complex_args`        | Integration | `gg commit -m "message"`  |
| `passthrough_preserves_exit_code` | Integration | Non-zero exits propagate  |
| `passthrough_preserves_colors`    | CLI         | TTY color detection works |
| `passthrough_hyphen_args`         | CLI         | `gg checkout -b feature`  |
| `passthrough_double_dash`         | CLI         | `gg -- file.txt` works    |


### CLI Argument Parsing


| Test Case             | Type | Description                    |
| --------------------- | ---- | ------------------------------ |
| `cli_help`            | CLI  | `gg --help` shows all commands |
| `cli_version`         | CLI  | `gg --version` works           |
| `cli_unknown_command` | CLI  | Falls through to git           |
| `cli_no_args`         | CLI  | `gg` alone shows git status    |


---

## TempRepo Helper

```rust
pub struct TempRepo {
    pub dir: TempDir,
    pub path: PathBuf,
}

impl TempRepo {
    pub fn new() -> Self;
    pub fn with_remote() -> Self;
    pub fn run_git(&self, args: &[&str]) -> Output;
    pub fn commit(&self, msg: &str);
    pub fn create_file(&self, name: &str, content: &str);
    pub fn create_branch(&self, name: &str);
    pub fn checkout(&self, branch: &str);
    pub fn current_branch(&self) -> String;
    pub fn commit_count(&self) -> usize;
}
```

---

## Implementation Priority

1. **Add dev-dependencies** to Cargo.toml
2. **Create `tests/common/temp_repo.rs**` - TempRepo helper
3. **Create CLI tests** - Quick wins with assert_cmd
4. **Create integration tests** - One command at a time
5. **Refactor for mockability** (optional) - If unit test isolation needed

---

## Expected Test Count


| Category              | Count   |
| --------------------- | ------- |
| Unit tests (existing) | 24      |
| Integration tests     | ~50     |
| CLI tests             | ~15     |
| **Total**             | **~89** |


