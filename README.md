# Git Gud


## Getting started

Git Gud is just an experimental cli tool I am working on that is a wrapper around git.
The tool provides some extra functionality ontop of git, and if a custom command is not found
it will default to using git. This tool requires git to be installed and have "git" in your PATH

To use this tool you can use the command `gg` for "git gud". 
This command is a wrapper around git so will contain all of the same functionality as git, for example you can run
`gg checkout -b some-branch` it will behave the same as `git checkout -b some-branch`

## Commands

- `gg status` | `gg s` => `git status`
- `gg clone ${url}` | `gg c ${url}` => `git clone ${url}`
- `gg push` | `gg p` => `git push` if on main or `git push --set-upstream branch-name` when on a branch


## License
MIT

