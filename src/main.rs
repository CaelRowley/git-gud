use clap::{Parser, Subcommand};

mod commands;
mod config;
mod git;
mod utils;

#[derive(Parser)]
#[command(name = "gg", version, about = "A smarter git wrapper")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Arguments passed to git when no subcommand matches
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    args: Vec<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Show status with grouped changes
    #[command(visible_alias = "s")]
    Status(commands::StatusArgs),

    /// Smart push with auto-upstream
    #[command(visible_alias = "p")]
    Push(commands::PushArgs),

    /// Sync branch with main/master
    Sync(commands::SyncArgs),

    /// Quick commit: stage and commit in one step
    #[command(visible_alias = "qc")]
    QuickCommit(commands::QuickCommitArgs),

    /// Amend the last commit
    Amend(commands::AmendArgs),

    /// Undo the last commit(s), keeping changes staged
    Undo(commands::UndoArgs),

    /// Open PR creation page in browser
    Pr(commands::PrArgs),

    /// Delete local branches that have been merged
    CleanBranches(commands::CleanBranchesArgs),

    /// List recently checked out branches
    Recent(commands::RecentArgs),

    /// Switch to a recent branch (interactive)
    Sw(commands::SwArgs),

    /// Show commits made today
    Today(commands::TodayArgs),

    /// Show commits since last workday (for standups)
    Standup(commands::StandupArgs),
}

fn main() {
    // Set up colors based on terminal/environment
    config::setup_colors();

    let cli = Cli::parse();

    let exit_code = match cli.command {
        Some(Commands::Status(args)) => commands::status::run(args),
        Some(Commands::Push(args)) => commands::push::run(args),
        Some(Commands::Sync(args)) => commands::sync::run(args),
        Some(Commands::QuickCommit(args)) => commands::quick_commit::run(args),
        Some(Commands::Amend(args)) => commands::amend::run(args),
        Some(Commands::Undo(args)) => commands::undo::run(args),
        Some(Commands::Pr(args)) => commands::pr::run(args),
        Some(Commands::CleanBranches(args)) => commands::clean_branches::run(args),
        Some(Commands::Recent(args)) => commands::recent::run(args),
        Some(Commands::Sw(args)) => commands::sw::run(args),
        Some(Commands::Today(args)) => commands::today::run(args),
        Some(Commands::Standup(args)) => commands::standup::run(args),
        None if cli.args.is_empty() => {
            // No args at all: show git status (common default)
            git::run(&["status"])
        }
        None => {
            // Unknown command: pass through to git with full colors
            git::passthrough(&cli.args)
        }
    };

    std::process::exit(exit_code);
}
