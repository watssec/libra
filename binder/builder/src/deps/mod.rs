use anyhow::Result;
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

impl DepAction {
    fn run_internal<R: Resolver, T: Dependency<R>>(self) -> Result<()> {
        let state: DepState<R, T> = DepState::new()?;
        match self {
            Self::Config => state.list_build_options()?,
            Self::Build { force } => state.build(force)?,
        }
        Ok(())
    }
}

#[derive(StructOpt)]
#[allow(clippy::upper_case_acronyms)]
pub enum DepArgs {
    LLVM(DepAction),
}

impl DepArgs {
    pub fn run(self) -> Result<()> {
        match self {
            Self::LLVM(action) => action.run_internal::<ResolverLLVM, DepLLVM>(),
        }
    }
}
