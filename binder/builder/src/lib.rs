mod deps;
mod pass;

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

use libra_shared::config::initialize;

pub use crate::deps::llvm::ResolverLLVM;
use crate::deps::DepArgs;
use crate::pass::PassArgs;

#[derive(Parser)]
#[clap(
    name = "libra-builder",
    about = "A custom builder for LLVM and LIBRA",
    rename_all = "kebab-case"
)]
struct Args {
    /// Subcommand
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// The dependencies
    #[command(subcommand)]
    Deps(DepArgs),
    /// The LLVM pass
    Pass(PassArgs),
}

/// Main entrypoint
pub fn entrypoint() -> Result<()> {
    initialize();

    // setup
    let args = Args::parse();
    let Args { command } = args;

    // run the command
    match command {
        Command::Deps(sub) => sub.run()?,
        Command::Pass(sub) => sub.build()?,
    }
    Ok(())
}

/// Utility function for exposing pass to others
pub fn artifact_for_pass() -> Result<PathBuf> {
    pass::artifact()
}
