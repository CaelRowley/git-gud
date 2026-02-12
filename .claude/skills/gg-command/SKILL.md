---
name: gg-command
description: Scaffold and implement new gg subcommands. Use when user asks to "add a command", "create a new gg subcommand", "implement a gg feature", or wants to extend git-gud with new CLI functionality.
---

# Add a New gg Command

Create and register a new gg subcommand: $ARGUMENTS

## Current Project State

- Existing commands: !`ls /home/cael/Git/git-gud/src/commands/*.rs | xargs -I{} basename {} .rs | grep -v mod`
- LFS subcommands: !`ls /home/cael/Git/git-gud/src/commands/lfs/*.rs 2>/dev/null | xargs -I{} basename {} .rs | grep -v mod`

## Steps

1. **Create command file** at `src/commands/<name>.rs`
   - Define `<Name>Args` struct with `#[derive(Args)]` from clap
   - Implement `pub fn run(args: <Name>Args) -> i32`
   - See [references/command-template.md](references/command-template.md) for the full template

2. **Register in `src/commands/mod.rs`**
   - Add `pub mod <name>;`
   - Add `pub use <name>::<Name>Args;`

3. **Add to CLI enum in `src/main.rs`**
   - Add variant to `Commands` enum with doc comment and optional `visible_alias`
   - Add match arm: `Some(Commands::<Name>(args)) => commands::<name>::run(args),`

4. **Build and test**: `cargo build && cargo test`

## Conventions

- Return `i32` exit code (0 = success)
- Use `eprintln!` for errors, `println!` for output
- Colors: `use colored::Colorize;` and `crate::config::Theme`
- Git calls: `crate::git::run()`, `crate::git::capture()`, `crate::git::passthrough()`
- Repo access: `crate::utils::repo::get_repo()`
- Aliases: `#[command(visible_alias = "x")]` on the enum variant
