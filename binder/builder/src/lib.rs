use std::path::PathBuf;

use anyhow::Result;
use simplelog::{ColorChoice, Config, LevelFilter, TermLogger, TerminalMode};
use structopt::StructOpt;

use libra_shared::config::PATH_STUDIO;

use crate::deps::DepArgs;

pub mod deps;
mod util;

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
    TermLogger::init(
        if verbose {
            LevelFilter::Debug
        } else {
            LevelFilter::Info
        },
        Config::default(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )?;

    // run the command
    match command {
        Command::Deps(sub) => sub.run(studio)?,
    }
    Ok(())
}
