---
name: rust-cli-patterns
description: Rust CLI conventions for git-gud. Apply when implementing CLI features, handling arguments with clap, managing terminal colors, structuring command modules, or using git2 for repository operations.
user-invocable: false
---

# Rust CLI Patterns for git-gud

## Project Structure

```
src/
├── main.rs           # CLI parsing (clap), command dispatch, git fallback
├── git.rs            # passthrough(), run(), run_sequence(), capture()
├── config.rs         # Theme colors, NO_COLOR support
├── commands/         # One file per command, each exports Args + run()
│   ├── mod.rs
│   └── lfs/          # LFS subcommand tree
└── utils/repo.rs     # get_repo(), get_branch_name(), is_main_branch()
```

## Core Patterns

**Argument parsing**: clap derive API with `#[derive(Args)]` structs. See [references/clap-patterns.md](references/clap-patterns.md).

**Colors**: `colored` crate + `Theme` struct. Respects `NO_COLOR` env var and TTY detection via `config::setup_colors()` called in `main()`.

**Error handling**: Return `i32` exit codes. Use `eprintln!("gg: {}", e)` for errors. For complex commands, use the two-function pattern:

```rust
pub fn run(args: CmdArgs) -> i32 {
    match run_inner(args) {
        Ok(()) => 0,
        Err(e) => { eprintln!("gg: {}", e); 1 }
    }
}
fn run_inner(args: CmdArgs) -> Result<(), Box<dyn std::error::Error>> {
    let repo = get_repo()?;
    // ... use ? freely
    Ok(())
}
```

**Dependencies**: clap 4 (derive), colored 2, git2 0.17. LFS adds: aws-sdk-s3, tokio, sha2, toml, serde, glob, dirs, thiserror, async-trait.
