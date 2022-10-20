use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Result};
use log::{info, warn};
use structopt::StructOpt;
use tempfile::tempdir;

use crate::deps::common::{DepState, Dependency};
use crate::deps::llvm::DepLLVM;

mod common;
mod llvm;

#[derive(StructOpt)]
pub enum DepCommand {
    /// Build the dependency
    Build {
        /// Temporary directory for the build process
        #[structopt(short, long)]
        tmpdir: Option<PathBuf>,

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
    command: DepCommand,
}

impl DepArgs {
    fn run_internal<T: Dependency>(self, studio: &Path) -> Result<()> {
        let Self {
            name: _,
            version,
            command,
        } = self;
        let mut state: DepState<T> = DepState::new(studio, version.as_deref())?;

        match command {
            DepCommand::Build {
                tmpdir,
                config,
                force,
            } => {
                // prepare the tmpdir first
                let tmpwks = match tmpdir {
                    None => Ok(tempdir()?),
                    Some(path) => {
                        if path.exists() {
                            if !force {
                                bail!("Tmpdir {} already exists", path.to_str().unwrap());
                            }
                            fs::remove_dir_all(&path)?;
                        }
                        fs::create_dir_all(&path)?;
                        Err(path)
                    }
                };
                let tmpdir = match &tmpwks {
                    Ok(dir) => dir.path(),
                    Err(path) => path.as_path(),
                };

                // case on config
                if config {
                    state.list_build_options(tmpdir)?;
                } else {
                    match state {
                        DepState::Scratch(scratch) => {
                            scratch.make(tmpdir)?;
                        }
                        DepState::Package(package) => {
                            if !force {
                                info!("Package already exists");
                            } else {
                                warn!("Force rebuilding package");
                                let scratch = package.destroy()?;
                                scratch.make(tmpdir)?;
                            }
                        }
                    }
                }

                // clean-up the temporary directory
                match tmpwks {
                    Ok(_) => {}
                    Err(path) => {
                        fs::remove_dir_all(path)?;
                    }
                }
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

