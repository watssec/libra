use std::path::{Path, PathBuf};

use anyhow::Result;
use structopt::StructOpt;

use libra_shared::config::PATH_STUDIO;
use libra_shared::logging;

use crate::deps::DepArgs;
use crate::pass::PassArgs;

pub mod deps;
pub mod pass;

#[derive(StructOpt)]
#[structopt(
    name = "libra-builder",
    about = "A custom builder for LLVM and LIBRA",
    rename_all = "kebab-case"
)]
struct Args {
    /// Studio directory
    #[structopt(short, long)]
    studio: Option<PathBuf>,

    /// Verbosity
    #[structopt(short, long)]
    verbose: bool,

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
    let Args {
        studio,
        verbose,
        command,
    } = args;
    let studio = studio.as_ref().unwrap_or(&PATH_STUDIO);

    // setup logging
    logging::setup(verbose)?;

    // run the command
    match command {
        Command::Deps(sub) => sub.run(studio)?,
        Command::Pass(sub) => sub.build(studio)?,
    }
    Ok(())
}

pub fn artifact_for_deps_llvm(studio: &Path, version: Option<&str>) -> Result<PathBuf> {
    deps::artifact_for_llvm(studio, version)
}

pub fn artifact_for_pass(studio: &Path, llvm_version: Option<&str>) -> Result<PathBuf> {
    pass::artifact(studio, llvm_version)
}
