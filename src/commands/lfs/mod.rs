//! LFS subcommands
//!
//! Provides commands for managing large file storage.

pub mod install;
pub mod pull;
pub mod push;
pub mod status;
pub mod track;
pub mod verify;

use clap::{Args, Subcommand};

pub use install::{InstallArgs, UninstallArgs};
pub use pull::PullArgs;
pub use push::PushArgs;
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

    /// Push LFS files to remote storage
    Push(PushArgs),

    /// Pull LFS files from remote storage
    Pull(PullArgs),

    /// Show LFS status
    Status(StatusArgs),

    /// Verify S3 configuration and connectivity
    Verify(VerifyArgs),
}

/// Run the LFS command
pub fn run(args: LfsArgs) -> i32 {
    match args.command {
        LfsCommand::Install(args) => install::run(args),
        LfsCommand::Uninstall(args) => install::run_uninstall(args),
        LfsCommand::Track(args) => track::run(args),
        LfsCommand::Untrack(args) => track::run_untrack(args),
        LfsCommand::Push(args) => push::run(args),
        LfsCommand::Pull(args) => pull::run(args),
        LfsCommand::Status(args) => status::run(args),
        LfsCommand::Verify(args) => verify::run(args),
    }
}
