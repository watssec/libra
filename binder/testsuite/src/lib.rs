use std::path::{Path, PathBuf};

use anyhow::{bail, Result};
use structopt::StructOpt;

use libra_shared::config::PATH_STUDIO;
use libra_shared::dep::{DepState, Dependency};
use libra_shared::logging;

use crate::llvm::DepLLVMTestSuite;

mod llvm;

#[derive(StructOpt)]
enum Command {
    /// Build the dependency
    Build {
        /// Temporary directory for the build process
        #[structopt(short, long)]
        tmpdir: bool,

        /// Run the configuration step instead of build
        #[structopt(short, long)]
        config: bool,

        /// Force the build to proceed
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

fn run_internal<T: Dependency>(
    studio: &Path,
    version: Option<&str>,
    command: Command,
) -> Result<()> {
    let state: DepState<T> = DepState::new(studio, version)?;
    match command {
        Command::Build {
            tmpdir,
            config,
            force,
        } => {
            state.build(tmpdir, config, force)?;
        }
    }
    Ok(())
}
