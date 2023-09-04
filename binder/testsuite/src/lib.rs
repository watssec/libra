use std::fs;
use std::path::Path;

use anyhow::{bail, Result};
use libra_shared::config::TMPDIR_IN_STUDIO;
use log::{info, warn};
use structopt::StructOpt;
use tempfile::tempdir;

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
                tmpdir: use_tmpdir,
                config,
                force,
            } => {
                // prepare the tmpdir first
                let tmpwks = if use_tmpdir {
                    Ok(tempdir()?)
                } else {
                    let path = studio.join(TMPDIR_IN_STUDIO);
                    if path.exists() {
                        if !force {
                            bail!("Tmpdir {} already exists", path.to_str().unwrap());
                        }
                        fs::remove_dir_all(&path)?;
                    }
                    fs::create_dir_all(&path)?;
                    Err(path)
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
            "llvm" => self.run_internal::<DepLLVMTestSuite>(studio),
            _ => bail!("Invalid deps name: {}", name),
        }
    }
}
