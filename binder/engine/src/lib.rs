use std::path::PathBuf;

pub use error::EngineError;

use crate::error::EngineResult;
use crate::flow::Workflow;

mod error;
mod flow;
mod ir;

/// Main entrypoint
pub fn analyze(
    depth: Option<usize>,
    flags: Vec<String>,
    inputs: Vec<PathBuf>,
    output: PathBuf,
) -> EngineResult<Vec<ir::bridge::module::Module>> {
    let flow = Workflow::new(flags, inputs, output);
    flow.execute(depth)
}
