mod common;
mod llvm_external;
mod llvm_internal;

use anyhow::Result;
use clap::{Parser, Subcommand};

use libra_shared::config::initialize;
use libra_shared::dep::{DepState, Dependency};

use crate::common::{TestCase, TestSuite};
use crate::llvm_external::{DepLLVMExternal, TestCaseExternal};
use crate::llvm_internal::{DepLLVMInternal, TestCaseInternal};

#[derive(Parser)]
#[clap(
    name = "libra-testsuite",
    about = "A driver for LIBRA test suites",
    rename_all = "kebab-case"
)]
struct Args {
    /// Test suite to run
    #[command(subcommand)]
    suite: Suite,
}

#[derive(Subcommand)]
enum Command {
    /// Print information about how to build the dependency
    Tweak,

    /// Build the test suite
    Build {
        /// Force the execution to proceed
        #[clap(short, long)]
        force: bool,
    },

    /// Run the test suite
    Run {
        /// Force the execution to proceed
        #[clap(short, long)]
        force: bool,

        /// Run selective test cases only
        #[clap(short, long)]
        selection: Vec<String>,
    },
}

impl Command {
    fn run_internal<C: TestCase, T: Dependency + TestSuite<C>>(self) -> Result<()> {
        let state: DepState<T> = DepState::new()?;
        match self {
            Self::Tweak => state.tweak()?,
            Self::Build { force } => state.build(force)?,
            Self::Run { force, selection } => T::run(force, selection)?,
        }
        Ok(())
    }
}

#[derive(Subcommand)]
enum Suite {
    #[command(subcommand)]
    External(Command),
    #[command(subcommand)]
    Internal(Command),
}

/// Main entrypoint
pub fn entrypoint() -> Result<()> {
    initialize();

    // setup
    let args = Args::parse();
    let Args { suite } = args;

    // run the subcommand
    match suite {
        Suite::External(command) => command.run_internal::<TestCaseExternal, DepLLVMExternal>(),
        Suite::Internal(command) => command.run_internal::<TestCaseInternal, DepLLVMInternal>(),
    }
}
