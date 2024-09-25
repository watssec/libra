use anyhow::Result;
use clap::Subcommand;

use libra_shared::dep::{DepState, Dependency};

use crate::deps::llvm::DepLLVM;

pub mod llvm;

#[derive(Subcommand)]
pub enum DepAction {
    /// Print information about how to build the dependency
    Tweak,

    /// Build the dependency
    Build {
        /// Force the build to proceed
        #[clap(short, long)]
        force: bool,
    },
}

impl DepAction {
    fn run_internal<T: Dependency>(self) -> Result<()> {
        let state: DepState<T> = DepState::new()?;
        match self {
            Self::Tweak => state.tweak()?,
            Self::Build { force } => state.build(force)?,
        }
        Ok(())
    }
}

#[derive(Subcommand)]
#[allow(clippy::upper_case_acronyms)]
pub enum DepArgs {
    #[command(subcommand)]
    LLVM(DepAction),
}

impl DepArgs {
    pub fn run(self) -> Result<()> {
        match self {
            Self::LLVM(action) => action.run_internal::<DepLLVM>(),
        }
    }
}
