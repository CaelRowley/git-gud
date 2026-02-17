//! LFS subcommands
//!
//! Provides commands for managing large file storage.

pub mod clean;
pub mod import;
pub mod install;
pub mod ls_files;
pub mod migrate;
pub mod prune;
pub mod pull;
pub mod push;
pub mod smudge;
pub mod status;
pub mod track;
pub mod verify;

use clap::{Args, Subcommand};

pub use clean::CleanArgs;
pub use import::ImportArgs;
pub use install::{InstallArgs, UninstallArgs};
pub use ls_files::LsFilesArgs;
pub use migrate::MigrateArgs;
pub use prune::PruneArgs;
pub use pull::PullArgs;
pub use push::PushArgs;
pub use smudge::SmudgeArgs;
pub use status::StatusArgs;
pub use track::{TrackArgs, UntrackArgs};
pub use verify::VerifyArgs;

/// LFS command arguments
#[derive(Args, Debug)]
pub struct LfsArgs {
    #[command(subcommand)]
    pub command: LfsCommand,
}

/// LFS subcommands
#[derive(Subcommand, Debug)]
pub enum LfsCommand {
    /// Install git hooks for automatic LFS operations
    Install(InstallArgs),

    /// Uninstall git hooks
    Uninstall(UninstallArgs),

    /// Track files matching a pattern with LFS
    Track(TrackArgs),

    /// Stop tracking files matching a pattern
    Untrack(UntrackArgs),

    /// Import existing large files into LFS (upload to S3, replace with pointers)
    Import(ImportArgs),

    /// Migrate from standard git-lfs to gg lfs
    Migrate(MigrateArgs),

    /// Push LFS files to remote storage
    Push(PushArgs),

    /// Pull LFS files from remote storage
    Pull(PullArgs),

    /// Show LFS status
    Status(StatusArgs),

    /// List LFS-tracked files
    LsFiles(LsFilesArgs),

    /// Prune old objects from the local LFS cache
    Prune(PruneArgs),

    /// Verify S3 configuration and connectivity
    Verify(VerifyArgs),

    /// Clean filter (used by git internally — converts file content to pointer)
    Clean(CleanArgs),

    /// Smudge filter (used by git internally — converts pointer to file content)
    Smudge(SmudgeArgs),
}

/// Run the LFS command
pub fn run(args: LfsArgs) -> i32 {
    match args.command {
        LfsCommand::Install(args) => install::run(args),
        LfsCommand::Uninstall(args) => install::run_uninstall(args),
        LfsCommand::Track(args) => track::run(args),
        LfsCommand::Untrack(args) => track::run_untrack(args),
        LfsCommand::Import(args) => import::run(args),
        LfsCommand::Migrate(args) => migrate::run(args),
        LfsCommand::Push(args) => push::run(args),
        LfsCommand::Pull(args) => pull::run(args),
        LfsCommand::Status(args) => status::run(args),
        LfsCommand::LsFiles(args) => ls_files::run(args),
        LfsCommand::Prune(args) => prune::run(args),
        LfsCommand::Verify(args) => verify::run(args),
        LfsCommand::Clean(args) => clean::run(args),
        LfsCommand::Smudge(args) => smudge::run(args),
    }
}
