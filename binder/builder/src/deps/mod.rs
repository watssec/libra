use anyhow::{bail, Result};
use structopt::StructOpt;

use libra_shared::dep::{DepState, Dependency, Resolver};

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

    /// Subcommand
    #[structopt(subcommand)]
    action: DepAction,
}

impl DepArgs {
    fn run_internal<R: Resolver, T: Dependency<R>>(self) -> Result<()> {
        let Self {
            name: _,
            action: command,
        } = self;
        let state: DepState<R, T> = DepState::new()?;

        match command {
            DepAction::Config => state.list_build_options()?,
            DepAction::Build { force } => state.build(force)?,
        }
        Ok(())
    }

    pub fn run(self) -> Result<()> {
        let name = self.name.as_str();
        match name {
            "llvm" => self.run_internal::<ResolverLLVM, DepLLVM>(),
            _ => bail!("Invalid deps name: {}", name),
        }
    }
}
