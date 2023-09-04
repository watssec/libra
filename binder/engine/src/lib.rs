use std::path::PathBuf;

pub use error::EngineError;

use crate::error::EngineResult;
use crate::flow::build_simple::FlowBuildSimple;
use crate::flow::fixedpoint::FlowFixedpoint;
use crate::flow::shared::Context;

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
    let ctxt = Context::new();

    // build
    let flow_build = FlowBuildSimple::new(&ctxt, inputs, output.clone(), flags);
    let merged_bc = flow_build.execute()?;

    // fixedpoint optimization
    let flow_fixedpoint = FlowFixedpoint::new(&ctxt, merged_bc, output, depth);
    flow_fixedpoint.execute()
}
