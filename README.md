# Git Gud

A smarter git CLI wrapper with sensible defaults. Any command not implemented falls back to git with full color and interactivity preserved.

## Install

- [Install git](https://git-scm.com/downloads)
- [Install rust and cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html)
- Run `cargo install git-gud`

## Commands

### `gg status` (alias: `s`)

Custom status view with grouped changes.

| Flag | Description |
|------|-------------|
| `-s, --short` | Show short format (delegates to `git status -s`) |

### `gg push` (alias: `p`)

Smart push with auto-upstream for branches.

- On main/master: runs `git push`
- On a branch: runs `git push --set-upstream origin <branch>` if no upstream is set

| Flag | Description |
|------|-------------|
| `-f, --force` | Force push with `--force-with-lease` |

### `gg sync`

Sync your branch with main/master.

- On main/master: runs `git pull --rebase`
- On a branch: stashes changes, checks out main, pulls, checks out your branch, rebases on main, pops stash

| Flag | Description |
|------|-------------|
| `--no-stash` | Don't stash changes before syncing |

### `gg quick-commit <message>` (alias: `qc`)

Stage and commit in one step.

| Flag | Description |
|------|-------------|
| `-A, --all` | Stage all changes including untracked files (`git add -A`) |
| `-p, --push` | Push after committing |

**Examples:**
```bash
gg qc "fix typo"              # Commit tracked changes only
gg qc "add feature" -A        # Commit everything including new files
gg qc "ready for review" -Ap  # Commit all and push
```

### Git Fallback

Any unrecognized command passes through to git with full colors preserved:

```bash
gg log --oneline -10    # → git log --oneline -10
gg checkout -b feature  # → git checkout -b feature
gg stash pop            # → git stash pop
```

![Alt text](assets/git-gud.png)

## License

MIT

