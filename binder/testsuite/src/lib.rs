use std::path::Path;

use anyhow::{bail, Result};
use structopt::StructOpt;

use libra_shared::dep::{DepState, Dependency};

use crate::llvm::DepLLVMTestSuite;

mod llvm;

#[derive(StructOpt)]
pub enum DepAction {
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
pub struct DepArgs {
    /// Name of the deps
    name: String,

    /// Version of the deps (tag or branch)
    #[structopt(short, long)]
    version: Option<String>,

    /// Subcommand
    #[structopt(subcommand)]
    action: DepAction,
}

impl DepArgs {
    fn run_internal<T: Dependency>(self, studio: &Path) -> Result<()> {
        let Self {
            name: _,
            version,
            action: command,
        } = self;
        let mut state: DepState<T> = DepState::new(studio, version.as_deref())?;

        match command {
            DepAction::Build {
                tmpdir,
                config,
                force,
            } => {
                state.build(tmpdir, config, force)?;
            }
        }
        Ok(())
    }

    pub fn run(self, studio: &Path) -> Result<()> {
        let name = self.name.as_str();
        match name {
            "llvm" => self.run_internal::<DepLLVMTestSuite>(studio),
            _ => bail!("Invalid deps name: {}", name),
        }
    }
}
