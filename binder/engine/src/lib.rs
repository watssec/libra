use std::path::PathBuf;

pub use error::EngineError;

use crate::error::EngineResult;
use crate::flow::Workflow;

mod error;
mod flow;
mod ir;

/// Main entrypoint
pub fn analyze(flags: Vec<String>, inputs: Vec<PathBuf>, output: PathBuf) -> EngineResult<()> {
    let flow = Workflow::new(flags, inputs, output);
    flow.execute(Some(0))
}
