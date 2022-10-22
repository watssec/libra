use std::path::PathBuf;

use crate::error::EngineResult;
use crate::flow::Workflow;

mod ir;
mod error;
mod flow;

/// Main entrypoint
pub fn analyze(inputs: Vec<PathBuf>, output: PathBuf) -> EngineResult<()> {
    let flow = Workflow::new(inputs, output);
    flow.execute()
}
