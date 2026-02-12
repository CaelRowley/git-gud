---
name: git-passthrough
description: Git command delegation patterns for git-gud. Apply when writing code that calls git, choosing between passthrough/run/capture, handling exit codes, or implementing git fallback behavior.
user-invocable: false
---

# Git Passthrough Patterns

When writing git-gud code that interacts with git, use the correct function from `src/git.rs`.

## Function Selection

| Scenario | Function | Why |
|----------|----------|-----|
| Unknown/fallback commands | `passthrough(&[String])` | Preserves colors, pager, interactivity |
| Internal git calls (add, commit) | `run(&[&str])` | Convenience wrapper, inherits stdio |
| Multiple sequential commands | `run_sequence(&[&[&str]])` | Stops on first failure |
| Need to parse git output | `capture(&[&str])` | Returns `Result<String, String>` |

## Key Rules

- **Never use `.output()`** to call git directly — it strips colors and breaks interactivity
- **Always use `.spawn()` with inherited stdio** for user-facing git output (already handled by `passthrough`)
- **Always propagate exit codes** — every command returns `i32`, chain with early return on non-zero
- **Default fallback**: unrecognized commands in `main.rs` pass through to git via `passthrough()`
- **`capture()` loses colors** — only use when you need to process the output string

## Exit Code Pattern

```rust
// Stop on first failure
let code = git::run(&["add", "-A"]);
if code != 0 { return code; }
let code = git::run(&["commit", "-m", &msg]);
if code != 0 { return code; }
```

Or use `run_sequence` for the same thing:

```rust
git::run_sequence(&[
    &["add", "-A"],
    &["commit", "-m", &msg],
])
```
