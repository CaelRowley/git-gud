# Git Gud

git-gud is a CLI tool for using git with some improved features. This is a **work in progress** but is still very **usable**, because it default to using `git` if there is no custom implementation.

## Install
- You will need to [install git](https://git-scm.com/downloads).
- You will also need to [install rust and cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html).
- Then run ```cargo install git-gud```

## Commands

Here is the list of custom commands

| `gg` | `git` |
| ----- | ----- |
| `gg status` <br/> `gg s` | `git status` |
| `gg clone ${url}` <br/> `gg c ${url}` | `git clone ${url}` |
| `gg push` <br/> `gg p` | When on main: `git push` <br/>When on a branch: `git push --set-upstream branch-name`|


If you run a command that is not implemented, for example `gg checkout -b some-branch` it will default to git and run the equivalent of `git checkout -b some-branch`

![Alt text](assets/git-gud.png)

## License
MIT

