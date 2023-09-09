mod common;
mod llvm_external;
mod llvm_lit;

use anyhow::{bail, Result};
use structopt::StructOpt;

use libra_shared::dep::{DepState, Dependency, Resolver};
use libra_shared::logging;

use crate::common::TestSuite;
use crate::llvm_external::{DepLLVMExternal, ResolverLLVMExternal};

#[derive(StructOpt)]
enum Command {
    /// Config the test suite
    Config,

    /// Build the test suite
    Build {
        /// Force the execution to proceed
        #[structopt(short, long)]
        force: bool,
    },

    /// Run the test suite
    Run {
        /// Force the execution to proceed
        #[structopt(short, long)]
        force: bool,
    },
}

#[derive(StructOpt)]
#[structopt(
    name = "libra-testsuite",
    about = "A driver for LIBRA test suites",
    rename_all = "kebab-case"
)]
struct Args {
    /// Verbosity
    #[structopt(short, long)]
    verbose: bool,

    /// Name of the test suite
    name: String,

    /// Subcommand
    #[structopt(subcommand)]
    command: Command,
}

/// Main entrypoint
pub fn entrypoint() -> Result<()> {
    let args = Args::from_args();
    let Args {
        verbose,
        name,
        command,
    } = args;
    // setup logging
    logging::setup(verbose)?;

    // run the subcommand
    match name.as_str() {
        "llvm" => run_internal::<ResolverLLVMExternal, DepLLVMExternal>(command)?,
        _ => bail!("Invalid deps name: {}", name),
    }

    Ok(())
}

fn run_internal<R: Resolver, T: Dependency<R> + TestSuite<R>>(command: Command) -> Result<()> {
    let state: DepState<R, T> = DepState::new()?;
    match command {
        Command::Config => state.list_build_options()?,
        Command::Build { force } => {
            state.build(force)?;
        }
        Command::Run { force } => {
            let (repo, resolver) = state.into_source_and_artifact()?;
            T::run(repo, resolver, force)?;
        }
    }
    Ok(())
}
