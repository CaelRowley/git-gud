# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Test Commands

```bash
cargo build                        # Debug build
cargo build --release              # Release build
cargo test                         # All tests (unit + integration)
cargo test --lib                   # Unit tests only
cargo test --test tests            # Integration tests only
cargo test test_status_clean_repo  # Single test by name
cargo test -- --nocapture          # Show println! output
```

The binary is named `gg` (defined in Cargo.toml `[[bin]]`).

## Architecture

**git-gud** (`gg`) is a git CLI wrapper in Rust. Unrecognized commands fall through to git with full TTY/color/pager preservation.

### Entry point (`src/main.rs`)

Clap derive API parses CLI into `Cli` struct. The `Commands` enum dispatches to command handlers. When no subcommand matches, args pass through to `git::passthrough()`. No args defaults to `git status`.

### Command pattern

Each command lives in `src/commands/<name>.rs` with:
- An `Args` struct using `#[derive(Args)]` (re-exported from `commands/mod.rs`)
- A `pub fn run(args: FooArgs) -> i32` that returns an exit code
- An inner `fn run_inner(args) -> Result<(), Box<dyn Error>>` that uses `?` for error handling

To add a new command: create the file, add `pub mod` + `pub use` in `commands/mod.rs`, add variant to `Commands` enum in `main.rs`, add match arm in `main()`.

### Git interaction (`src/git.rs`)

Four functions for calling git:

| Function | When to use |
|----------|-------------|
| `passthrough(&[String])` | Unknown commands — preserves TTY/colors/pager |
| `run(&[&str])` | Internal git calls (add, commit) — preserves TTY |
| `run_sequence(&[&[&str]])` | Chain commands, stop on first failure |
| `capture(&[&str])` | Need to parse output — returns `Result<String, String>`, loses colors |

All use `.spawn()` + inherited stdio except `capture()` which uses `.output()`.

### Theming (`src/config.rs`)

Central `Theme` struct with colors for staged/modified/untracked/deleted/branch/command. Respects `NO_COLOR` env var and TTY detection via `config::setup_colors()`.

### Repository utilities (`src/utils/repo.rs`)

`get_repo()`, `get_branch_name()`, `is_main_branch()`, `get_main_branch_name()` — wraps `git2` for programmatic repo access.

### LFS subsystem (`src/lfs/` + `src/commands/lfs/`)

Custom git-lfs alternative using AWS S3. `src/lfs/` has the implementation (config, pointer files, scanner, cache, S3 storage trait). `src/commands/lfs/` has the CLI command handlers. Config lives in `.gg/lfs.toml`. Uses tokio for async S3 operations.

### Test infrastructure (`tests/`)

Single entry point `tests/tests.rs` imports `cli/` and `integration/` modules. `TempRepo` fixture (`tests/common/temp_repo.rs`) provides a temporary git repo with helpers: `repo.gg(&["cmd"])` runs the gg binary, `repo.git_output(&["args"])` runs git, plus file/commit/branch manipulation methods. `TempRepo::with_remote()` creates a repo with a bare remote for push tests.
