# Clap Patterns for git-gud

## CLI Root (`src/main.rs`)

```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "gg", version, about = "A smarter git wrapper")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Arguments passed to git when no subcommand matches
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    args: Vec<String>,
}
```

The `args` field with `trailing_var_arg` captures everything when no subcommand matches, enabling git fallback.

## Subcommand Enum

```rust
#[derive(Subcommand)]
enum Commands {
    /// Show status with grouped changes
    #[command(visible_alias = "s")]
    Status(StatusArgs),

    /// Smart push with auto-upstream
    #[command(visible_alias = "p")]
    Push(PushArgs),

    /// Stage, commit, and optionally push
    #[command(visible_alias = "qc")]
    QuickCommit(QuickCommitArgs),

    /// LFS alternative using S3
    Lfs(LfsArgs),
}
```

## Dispatch

```rust
let exit_code = match cli.command {
    Some(Commands::Status(args)) => commands::status::run(args),
    // ...
    None if cli.args.is_empty() => git::run(&["status"]),
    None => git::passthrough(&cli.args),
};
std::process::exit(exit_code);
```

## Argument Types

### Boolean flag
```rust
#[arg(short, long)]
pub force: bool,
```

### Flag with short alias
```rust
#[arg(short = 'A', long)]
pub all: bool,
```

### Positional (required)
```rust
/// Commit message
pub message: String,
```

### Positional (optional)
```rust
/// Target branch
pub branch: Option<String>,
```

### With default value
```rust
#[arg(short, long, default_value_t = 10)]
pub count: usize,
```

### With default of 1
```rust
/// Number of commits to undo
#[arg(default_value_t = 1)]
pub count: u32,
```

## Nested Subcommands (LFS pattern)

```rust
#[derive(Args)]
pub struct LfsArgs {
    #[command(subcommand)]
    pub command: LfsCommand,
}

#[derive(Subcommand)]
pub enum LfsCommand {
    Install(InstallArgs),
    Track(TrackArgs),
    Push(PushArgs),
    Pull(PullArgs),
    Status(StatusArgs),
    Verify(VerifyArgs),
}
```
