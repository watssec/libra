mod deps;
mod pass;

use std::path::PathBuf;

use anyhow::Result;
use structopt::StructOpt;

use libra_shared::logging;

pub use crate::deps::llvm::ResolverLLVM;
use crate::deps::DepArgs;
use crate::pass::PassArgs;

#[derive(StructOpt)]
#[structopt(
    name = "libra-builder",
    about = "A custom builder for LLVM and LIBRA",
    rename_all = "kebab-case"
)]
struct Args {
    /// Verbosity
    #[structopt(short, long)]
    verbose: Option<usize>,

    /// Subcommand
    #[structopt(subcommand)]
    command: Command,
}

#[derive(StructOpt)]
enum Command {
    /// The dependencies
    #[structopt(name = "deps")]
    Deps(DepArgs),
    /// The LLVM pass
    #[structopt(name = "pass")]
    Pass(PassArgs),
}

/// Main entrypoint
pub fn entrypoint() -> Result<()> {
    let args = Args::from_args();
    let Args { verbose, command } = args;
    // setup logging
    logging::setup(verbose)?;

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
