mod common;
mod llvm_external;
mod llvm_internal;

use anyhow::Result;
use structopt::StructOpt;

use libra_shared::config::initialize;
use libra_shared::dep::{DepState, Dependency, Resolver};

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

impl Command {
    fn run_internal<C: TestCase, R: Resolver, T: Dependency<R> + TestSuite<C, R>>(
        self,
    ) -> Result<()> {
        let state: DepState<R, T> = DepState::new()?;
        match self {
            Self::Config => state.list_build_options()?,
            Self::Build { force } => {
                state.build(force)?;
            }
            Self::Run { force, selection } => {
                let (repo, resolver) = state.into_source_and_artifact()?;
                T::run(repo, resolver, force, selection)?;
            }
        }
        Ok(())
    }
}

#[derive(StructOpt)]
enum Suite {
    External(Command),
    Internal(Command),
}

#[derive(StructOpt)]
#[structopt(
    name = "libra-testsuite",
    about = "A driver for LIBRA test suites",
    rename_all = "kebab-case"
)]
struct Args {
    /// Test suite to run
    #[structopt(subcommand)]
    suite: Suite,
}

/// Main entrypoint
pub fn entrypoint() -> Result<()> {
    let args = Args::from_args();
    let Args { suite } = args;
    // setup
    initialize();

    // run the subcommand
    match suite {
        Suite::External(command) => {
            command.run_internal::<TestCaseExternal, ResolverLLVMExternal, DepLLVMExternal>()
        }
        Suite::Internal(command) => {
            command.run_internal::<TestCaseInternal, ResolverLLVMInternal, DepLLVMInternal>()
        }
    }
}
