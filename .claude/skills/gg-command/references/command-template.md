# Command Template

## Command File (`src/commands/<name>.rs`)

```rust
use clap::Args;
use crate::git;
use crate::utils::repo::get_repo;

#[derive(Args)]
pub struct <Name>Args {
    /// Positional argument
    pub target: String,

    /// Optional positional
    pub directory: Option<String>,

    /// Boolean flag
    #[arg(short, long)]
    pub force: bool,

    /// Flag with default value
    #[arg(short, long, default_value_t = 10)]
    pub count: usize,
}

pub fn run(args: <Name>Args) -> i32 {
    // Option A: Direct git delegation
    git::run(&["<git-subcommand>", &args.target])

    // Option B: Use git2 repo
    let repo = match get_repo() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("gg: {}", e);
            return 1;
        }
    };
    // ... repo operations ...
    0
}
```

## Registration in `src/commands/mod.rs`

```rust
pub mod <name>;
pub use <name>::<Name>Args;
```

## CLI Enum in `src/main.rs`

```rust
#[derive(Subcommand)]
enum Commands {
    /// Help text shown in --help
    #[command(visible_alias = "x")]
    <Name>(<Name>Args),
}

// In the match block:
Some(Commands::<Name>(args)) => commands::<name>::run(args),
```

## Git Function Reference

| Function | Use when |
|----------|----------|
| `git::passthrough(&[String])` | Delegating to git with full TTY (colors, pager, interactivity) |
| `git::run(&[&str])` | Running git internally, no output capture needed |
| `git::run_sequence(&[&[&str]])` | Multiple git commands, stop on first failure |
| `git::capture(&[&str])` | Need to parse git output (loses colors) |

## Repo Utilities (`crate::utils::repo`)

| Function | Returns |
|----------|---------|
| `get_repo()` | `Result<Repository, git2::Error>` |
| `get_branch_name(&repo)` | `Option<String>` |
| `is_main_branch(branch)` | `bool` (matches "main" or "master") |
| `get_main_branch_name(&repo)` | `&'static str` ("main" or "master") |

## Color Usage

```rust
use colored::Colorize;
use crate::config::Theme;

let theme = Theme::default();
println!("{}", "text".color(theme.branch));
println!("{}", "bold text".bold().green());
```

Theme colors: `staged` (green), `modified` (yellow), `untracked` (red), `deleted` (red), `branch` (cyan), `command` (white).
