---
name: Git-gud CLI Refactor
overview: Refactor git-gud into a properly architected CLI wrapper with transparent git fallback, modular command structure, and improved color handling.
todos:
  - id: git-passthrough
    content: Create src/git.rs with passthrough_to_git() using Stdio::inherit() for native colors
    status: pending
  - id: fix-default
    content: Update default.rs to use new passthrough (removes 'Running default command' prefix)
    status: pending
  - id: add-clap
    content: Add clap to Cargo.toml and refactor main.rs with proper CLI parsing
    status: pending
  - id: commands-module
    content: Create src/commands/ directory structure and migrate existing commands
    status: pending
  - id: config-colors
    content: Create src/config.rs with Theme struct and NO_COLOR support
    status: pending
  - id: utils-module
    content: Create src/utils/repo.rs with shared helpers (get_repo, get_branch_name, etc.)
    status: pending
  - id: new-commands
    content: Implement additional utility commands (qc, amend, undo, etc.)
    status: pending
isProject: false
---

# Git-gud CLI Refactor Plan

## Current State Analysis

Your current implementation has:

- Basic command routing in `main.rs` with match statement
- Custom commands: `clone`, `status`, `push`, `sync`
- Fallback to git via `default.rs`
- Hardcoded colors using the `colored` crate
- Duplicated command execution logic across modules

**Key issues identified:**

1. Git fallback uses `.output()` which captures stdout/stderr, losing git's native colors
2. Duplicated `run_command` helper in multiple files
3. No proper CLI argument parsing (will panic on `gg` with no args)
4. Hardcoded color values instead of respecting terminal/git config

---

## 1. Proper Git Fallback with Native Colors

**Problem:** Using `Command::new("git").output()` captures output as bytes, stripping git's terminal colors.

**Solution:** Use `.spawn()` with inherited stdio instead of `.output()`:

```rust
use std::process::{Command, Stdio};

pub fn passthrough_to_git(args: &[String]) -> std::io::Result<i32> {
    let mut child = Command::new("git")
        .args(args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()?;
    
    let status = child.wait()?;
    Ok(status.code().unwrap_or(1))
}
```

This:

- Preserves git's native color output (git auto-detects TTY)
- Maintains interactive features (e.g., `git add -p`)
- Properly forwards exit codes

**File:** Create new `[src/git.rs](src/git.rs)` module for git interaction utilities.

---

## 2. Proper CLI Architecture with clap

**Add clap for argument parsing** - update `Cargo.toml`:

```toml
[dependencies]
clap = { version = "4", features = ["derive"] }
```

**New main.rs structure:**

```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "gg", about = "A smarter git wrapper")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
    
    /// Pass remaining args to git directly
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    git_args: Vec<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Custom status with grouping
    Status(StatusArgs),
    /// Smart push with auto-upstream
    Push(PushArgs),
    /// Sync branch with main/master
    Sync(SyncArgs),
    // Add more custom commands here
}

fn main() {
    let cli = Cli::parse();
    
    let exit_code = match cli.command {
        Some(Commands::Status(args)) => commands::status::run(args),
        Some(Commands::Push(args)) => commands::push::run(args),
        Some(Commands::Sync(args)) => commands::sync::run(args),
        None => git::passthrough(&cli.git_args),
    };
    
    std::process::exit(exit_code);
}
```

---

## 3. Modular Command Structure

**Proposed directory structure:**

```
src/
├── main.rs              # CLI parsing, command dispatch
├── git.rs               # Git passthrough utilities
├── config.rs            # Color/config management
├── commands/
│   ├── mod.rs           # Re-exports all commands
│   ├── status.rs        # gg status
│   ├── push.rs          # gg push  
│   ├── sync.rs          # gg sync
│   └── quick_commit.rs  # Example: gg qc "message"
└── utils/
    ├── mod.rs
    └── repo.rs          # Repository helpers (get current branch, etc.)
```

**Command trait pattern for consistency:**

```rust
// src/commands/mod.rs
pub trait GgCommand {
    fn run(&self) -> i32;
}

// Each command implements this, making adding new commands straightforward
```

**Adding a new command requires:**

1. Create `src/commands/new_command.rs`
2. Add struct + implement logic
3. Add variant to `Commands` enum in `main.rs`
4. Add match arm in dispatch

---

## 4. Color Configuration

**Replace hardcoded colors with configurable theming:**

```rust
// src/config.rs
use colored::Color;

pub struct Theme {
    pub staged: Color,
    pub modified: Color,
    pub untracked: Color,
    pub deleted: Color,
    pub branch: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            staged: Color::Green,
            modified: Color::Yellow,
            untracked: Color::Red,
            deleted: Color::Red,
            branch: Color::Cyan,
        }
    }
}
```

**Respect `NO_COLOR` and `GG_COLOR` environment variables:**

```rust
pub fn colors_enabled() -> bool {
    std::env::var("NO_COLOR").is_err() 
        && std::env::var("GG_COLOR").map(|v| v != "0").unwrap_or(true)
}
```

---

## 5. Useful Wrapper Command Ideas


| Command             | Description                                       |
| ------------------- | ------------------------------------------------- |
| `gg qc "msg"`       | Quick commit: `git add -A && git commit -m "msg"` |
| `gg amend`          | Amend last commit without editing message         |
| `gg undo`           | Undo last commit, keep changes staged             |
| `gg wip`            | Commit all as "WIP" (work in progress)            |
| `gg unwip`          | Undo if last commit was WIP                       |
| `gg pr`             | Open PR creation URL in browser                   |
| `gg clean-branches` | Delete merged local branches                      |
| `gg recent`         | List recently checked out branches                |
| `gg sw`             | Interactive branch switcher (fzf-style)           |
| `gg fixup`          | Interactive fixup for recent commits              |
| `gg today`          | Show commits made today                           |
| `gg standup`        | Show your commits from last workday               |


---

## Implementation Order

1. **Create `src/git.rs**` - passthrough utility with proper stdio inheritance
2. **Fix `src/default.rs**` - use new passthrough (immediate improvement)
3. **Add clap** - proper argument parsing
4. **Restructure to `src/commands/**` - modular layout
5. **Add `src/config.rs**` - theme configuration
6. **Migrate existing commands** - status, push, sync, clone
7. **Add new utility commands** - qc, amend, undo, etc.

