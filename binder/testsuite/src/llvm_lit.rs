use std::path::{Path, PathBuf};
use std::{env, fs};

use anyhow::{bail, Result};
use libra_engine::error::{EngineError, EngineResult};
use libra_engine::flow::fixedpoint::FlowFixedpoint;
use libra_engine::flow::shared::Context;
use log::{debug, warn};

use libra_shared::compile_db::ClangCommand;

pub enum LLVMTestResult {
    Success,
    Failure(String),
}

pub struct LLVMTestCase {
    name: String,
    path: PathBuf,
    command: ClangCommand,
}

impl LLVMTestCase {
    pub fn new(name: String, path: PathBuf, command: ClangCommand) -> Self {
        Self {
            name,
            path,
            command,
        }
    }

    /// Run the test case through libra workflow (internal)
    fn run_libra_internal(
        &self,
        ctxt: &Context,
        workdir: &Path,
    ) -> Result<Option<EngineResult<()>>> {
        let Self {
            name,
            path: _,
            command,
        } = self;

        // TODO: support c++
        if command.is_cpp {
            return Ok(None);
        }

        // retrieve input
        let inputs = command.inputs();
        if inputs.len() != 1 {
            // NOTE: this is true because we use SingleSource tests only
            bail!("expect one and only one input");
        }
        let input = inputs.into_iter().next().unwrap();

        // report progress
        debug!("running test case: {}", name);

        // prepare output directory
        let output_dir = workdir.join(name);
        fs::create_dir_all(&output_dir)?;

        // temporarily change directory
        let cursor = env::current_dir()?;
        env::set_current_dir(&command.workdir)?;

        // workflow
        let result = libra_workflow(ctxt, command, Path::new(input), &output_dir);

        // clean-up
        env::set_current_dir(cursor)?;
        Ok(Some(result))
    }

    pub fn run_libra(&self, ctxt: &Context, workdir: &Path) -> Option<(String, LLVMTestResult)> {
        match self.run_libra_internal(ctxt, workdir) {
            Ok(None) => None,
            Ok(Some(_)) => Some((self.name.to_string(), LLVMTestResult::Success)),
            Err(e) => {
                warn!(
                    "test {} with path {} failed: {}",
                    self.name,
                    self.path.to_string_lossy(),
                    e
                );
                Some((
                    self.name.to_string(),
                    LLVMTestResult::Failure(e.to_string()),
                ))
            }
        }
    }
}

/// Run libra engine
fn libra_workflow(
    ctxt: &Context,
    command: &ClangCommand,
    input: &Path,
    output: &Path,
) -> EngineResult<()> {
    // compile
    let bc_init = output.join("init.bc");
    ctxt.compile_to_bitcode(input, &bc_init, command.gen_args_for_libra())
        .map_err(|e| EngineError::CompilationError(format!("Error during clang: {}", e)))?;
    ctxt.disassemble_in_place(&bc_init)
        .map_err(|e| EngineError::CompilationError(format!("Error during disas: {}", e)))?;

    // fixedpoint
    let flow_fp = FlowFixedpoint::new(ctxt, bc_init, output.to_path_buf(), None);
    flow_fp.execute()?;

    // done with everything
    Ok(())
}
