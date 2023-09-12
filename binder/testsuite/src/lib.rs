mod common;
mod llvm_external;
mod llvm_internal;

use anyhow::{bail, Result};
use structopt::StructOpt;

use libra_shared::dep::{DepState, Dependency, Resolver};
use libra_shared::logging;

use crate::common::{TestCase, TestSuite};
use crate::llvm_external::{DepLLVMExternal, ResolverLLVMExternal, TestCaseExternal};
use crate::llvm_internal::{DepLLVMInternal, ResolverLLVMInternal, TestCaseInternal};

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

        /// Run selective test cases only
        #[structopt(short, long)]
        selection: Vec<String>,
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
    verbose: Option<usize>,

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
        "external" => {
            run_internal::<TestCaseExternal, ResolverLLVMExternal, DepLLVMExternal>(command)?
        }
        "internal" => {
            run_internal::<TestCaseInternal, ResolverLLVMInternal, DepLLVMInternal>(command)?
        }
        _ => bail!("Invalid deps name: {}", name),
    }

    Ok(())
}

fn run_internal<C: TestCase, R: Resolver, T: Dependency<R> + TestSuite<C, R>>(
    command: Command,
) -> Result<()> {
    let state: DepState<R, T> = DepState::new()?;
    match command {
        Command::Config => state.list_build_options()?,
        Command::Build { force } => {
            state.build(force)?;
        }
        Command::Run { force, selection } => {
            let (repo, resolver) = state.into_source_and_artifact()?;
            T::run(repo, resolver, force, selection)?;
        }
    }
    Ok(())
}
