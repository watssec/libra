use std::path::{Path, PathBuf};

use anyhow::{bail, Result};
use structopt::StructOpt;

use libra_shared::dep::{DepState, Dependency};

use crate::deps::llvm::DepLLVM;

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
        let state: DepState<T> = DepState::new(studio, version.as_deref())?;

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
            "llvm" => self.run_internal::<DepLLVM>(studio),
            _ => bail!("Invalid deps name: {}", name),
        }
    }
}

/// Retrieve the paths of dependencies
fn get_artifact_path<T: Dependency>(studio: &Path, version: Option<&str>) -> Result<PathBuf> {
    let path = match DepState::<T>::new(studio, version)? {
        DepState::Scratch(_) => bail!("Package not ready"),
        DepState::Package(pkg) => pkg.artifact_path().to_path_buf(),
    };
    Ok(path)
}

pub fn artifact_for_llvm(studio: &Path, version: Option<&str>) -> Result<PathBuf> {
    get_artifact_path::<DepLLVM>(studio, version)
}
