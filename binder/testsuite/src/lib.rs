use std::path::{Path, PathBuf};

use anyhow::{bail, Result};
use structopt::StructOpt;

use libra_shared::config::PATH_STUDIO;
use libra_shared::dep::{DepState, Dependency};
use libra_shared::logging;

use crate::common::TestSuite;
use crate::llvm::DepLLVMTestSuite;

mod common;
mod llvm;

#[derive(StructOpt)]
enum Command {
    /// Config the test suite
    Config,

    /// Build the test suite
    Build {
        /// Force the build to proceed
        #[structopt(short, long)]
        force: bool,
    },

    /// Run the test suite
    Run,
}

#[derive(StructOpt)]
#[structopt(
    name = "libra-testsuite",
    about = "A driver for LIBRA test suites",
    rename_all = "kebab-case"
)]
struct Args {
    /// Studio directory
    #[structopt(short, long)]
    studio: Option<PathBuf>,

    /// Verbosity
    #[structopt(short, long)]
    verbose: bool,

    /// Name of the test suite
    name: String,

    /// Version of the test suite (tag or branch)
    #[structopt(long)]
    version: Option<String>,

    /// Subcommand
    #[structopt(subcommand)]
    command: Command,
}

/// Main entrypoint
pub fn entrypoint() -> Result<()> {
    let args = Args::from_args();
    let Args {
        studio,
        verbose,
        name,
        version,
        command,
    } = args;
    let studio = studio.as_ref().unwrap_or(&PATH_STUDIO);

    // setup logging
    logging::setup(verbose)?;

    // run the subcommand
    match name.as_str() {
        "llvm" => run_internal::<DepLLVMTestSuite>(studio, version.as_deref(), command)?,
        _ => bail!("Invalid deps name: {}", name),
    }

    Ok(())
}

fn run_internal<T: Dependency + TestSuite>(
    studio: &Path,
    version: Option<&str>,
    command: Command,
) -> Result<()> {
    let state: DepState<T> = DepState::new(studio, version)?;
    match command {
        Command::Config => state.list_build_options()?,
        Command::Build { force } => {
            state.build(force)?;
        }
        Command::Run => match state {
            DepState::Scratch(_) => bail!("package not ready"),
            DepState::Package(pkg) => {
                T::run(pkg.git_repo(), pkg.artifact_path())?;
            }
        },
    }
    Ok(())
}
