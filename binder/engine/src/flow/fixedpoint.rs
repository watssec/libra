use std::path::PathBuf;

use log::debug;

use crate::error::EngineError;
use crate::error::EngineResult;
use crate::flow::shared::Context;
use crate::ir::bridge;

pub struct FlowFixedpoint<'a> {
    /// Context manager
    ctxt: &'a Context,
    /// Source bitcode file
    input: PathBuf,
    /// Output directory of the process
    output: PathBuf,
    /// Depth of loop (if set)
    depth: Option<usize>,
}

/// Entrypoints
impl<'a> FlowFixedpoint<'a> {
    pub fn new(ctxt: &'a Context, input: PathBuf, output: PathBuf, depth: Option<usize>) -> Self {
        Self {
            ctxt,
            input,
            output,
            depth,
        }
    }

    pub fn execute(self) -> EngineResult<Vec<bridge::module::Module>> {
        let Self {
            ctxt,
            input,
            output,
            depth,
        } = self;

        // sanity checking
        ctxt.opt_verify(&input).map_err(|e| {
            EngineError::CompilationError(format!("Error during opt -passes=verify: {}", e))
        })?;
        ctxt.disassemble_in_place(&input)
            .map_err(|e| EngineError::CompilationError(format!("Error during disas: {}", e)))?;
        debug!("[0] sanity checked");

        // baseline loading
        let mut history = vec![];
        let baseline = ctxt.load(&input)?;
        history.push((input, baseline));
        debug!("[0] baseline recorded");

        // optimization until a fixedpoint
        loop {
            // limit the number of iterations if requested
            if depth.map_or(false, |limit| history.len() > limit) {
                break;
            }

            let (last_path, last_ir) = history.last().unwrap();
            let step = history.len();

            // optimization
            let this_path = output.join(format!("step-{}.bc", step));
            ctxt.opt_pipeline(last_path, &this_path, "default<O3>")
                .map_err(|e| EngineError::CompilationError(format!("Error during opt: {}", e)))?;
            ctxt.disassemble_in_place(&this_path)
                .map_err(|e| EngineError::CompilationError(format!("Error during disas: {}", e)))?;
            debug!("[{}] optimization done", step);

            // loading
            let optimized = ctxt.load(&this_path)?;
            debug!("[{}] module recorded", step);

            // check for fixedpoint
            if last_ir == &optimized {
                break;
            }
            history.push((this_path, optimized));
        }
        debug!("[{}] fixedpoint optimization done", history.len());

        // return the full optimization trace
        let trace = history.into_iter().map(|(_, m)| m).collect();
        Ok(trace)
    }
}
