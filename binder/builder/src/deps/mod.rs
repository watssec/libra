use std::path::{Path, PathBuf};

use anyhow::{bail, Result};
use structopt::StructOpt;

use libra_shared::dep::{DepState, Dependency};

use crate::deps::llvm::{DepLLVM, ResolverLLVM};

pub mod llvm;

#[derive(StructOpt)]
pub enum DepAction {
    /// Config the dependency
    Config,

    /// Build the dependency
    Build {
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
    fn run_internal<R, T: Dependency<R>>(self, studio: &Path) -> Result<()> {
        let Self {
            name: _,
            version,
            action: command,
        } = self;
        let state: DepState<R, T> = DepState::new(studio, version.as_deref())?;

        match command {
            DepAction::Config => state.list_build_options()?,
            DepAction::Build { force } => state.build(force)?,
        }
        Ok(())
    }

    pub fn run(self, studio: &Path) -> Result<()> {
        let name = self.name.as_str();
        match name {
            "llvm" => self.run_internal::<ResolverLLVM, DepLLVM>(studio),
            _ => bail!("Invalid deps name: {}", name),
        }
    }
}

/// Retrieve the paths of dependencies
fn get_artifact_path<R, T: Dependency<R>>(studio: &Path, version: Option<&str>) -> Result<PathBuf> {
    let path = match DepState::<R, T>::new(studio, version)? {
        DepState::Scratch(_) => bail!("package not ready"),
        DepState::Package(pkg) => pkg.artifact_path().to_path_buf(),
    };
    Ok(path)
}

pub fn artifact_for_llvm(studio: &Path, version: Option<&str>) -> Result<PathBuf> {
    get_artifact_path::<ResolverLLVM, DepLLVM>(studio, version)
}
