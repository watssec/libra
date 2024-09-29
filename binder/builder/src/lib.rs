pub mod deps;
pub mod pass;

use anyhow::Result;
use clap::{Parser, Subcommand};

use crate::deps::llvm::DepLLVM;
use crate::pass::DepOracle;
use libra_shared::config::initialize;
use libra_shared::dep::{DepState, Dependency};

#[derive(Parser)]
#[clap(
    name = "libra-builder",
    about = "A custom builder for LLVM and LIBRA",
    rename_all = "kebab-case"
)]
struct Args {
    /// Subcommand
    #[command(subcommand)]
    command: DepCommand,
}

#[derive(Subcommand)]
pub enum DepAction {
    /// Print information about how to build the dependency
    Tweak,

    /// Build the dependency
    Build {
        /// Force the build to proceed
        #[clap(short, long)]
        force: bool,
    },
}

impl DepAction {
    fn run_internal<T: Dependency>(self) -> Result<()> {
        let state: DepState<T> = DepState::new()?;
        match self {
            Self::Tweak => state.tweak()?,
            Self::Build { force } => state.build(force)?,
        }
        Ok(())
    }
}

#[derive(Subcommand)]
#[allow(clippy::upper_case_acronyms)]
pub enum DepCommand {
    #[command(subcommand)]
    LLVM(DepAction),
    #[command(subcommand)]
    Oracle(DepAction),
}

impl DepCommand {
    pub fn run(self) -> Result<()> {
        match self {
            Self::LLVM(action) => action.run_internal::<DepLLVM>(),
            Self::Oracle(action) => action.run_internal::<DepOracle>(),
        }
    }
}

/// Main entrypoint
pub fn entrypoint() -> Result<()> {
    initialize();

    // setup
    let args = Args::parse();
    let Args { command } = args;

    // run the command
    command.run()?;
    Ok(())
}
