use crate::error::EngineResult;
use crate::EngineError;
use std::path::PathBuf;

use crate::flow::shared::Context;

/// Default flags to be sent to clang
static PRESET_CLANG_FLAGS: [&str; 8] = [
    // attach debug symbol
    "-g",
    // targeting the C language
    "--language",
    "c",
    // feature selection
    "-std=gnu17",
    "-Wno-c2x-extensions",
    // disable unsupported features
    "-fno-vectorize",
    // allow subsequent optimizations
    "-Xclang",
    "-disable-O0-optnone",
];

pub struct FlowBuildSimple<'a> {
    /// Context manager
    ctxt: &'a Context,
    /// Source file
    inputs: Vec<PathBuf>,
    /// Workspace for the analysis
    output: PathBuf,
    /// Flags (to be sent to Clang)
    flags: Vec<String>,
}

impl<'a> FlowBuildSimple<'a> {
    pub fn new(
        ctxt: &'a Context,
        inputs: Vec<PathBuf>,
        output: PathBuf,
        flags: Vec<String>,
    ) -> Self {
        let all_flags = PRESET_CLANG_FLAGS
            .iter()
            .map(|i| i.to_string())
            .chain(flags.into_iter())
            .collect();
        Self {
            ctxt,
            inputs,
            output,
            flags: all_flags,
        }
    }

    pub fn execute(self) -> EngineResult<PathBuf> {
        let Self {
            ctxt,
            inputs,
            output,
            flags,
        } = self;

        // compilation
        let mut init_bc_files = vec![];
        for (i, src) in inputs.iter().enumerate() {
            let bc_path = output.join(format!("init-{}.bc", i));
            ctxt.compile_to_bitcode(src, &bc_path, flags.iter().map(|i| i.as_str()))
                .map_err(|e| EngineError::CompilationError(format!("Error during clang: {}", e)))?;
            ctxt.disassemble_in_place(&bc_path)
                .map_err(|e| EngineError::CompilationError(format!("Error during disas: {}", e)))?;
            init_bc_files.push(bc_path);
        }

        // linking
        let path_refs: Vec<_> = init_bc_files.iter().map(|p| p.as_path()).collect();
        let merged_bc_path = output.join("merged.bc");
        ctxt.link_bitcode(&path_refs, &merged_bc_path)
            .map_err(|e| EngineError::CompilationError(format!("Error during llvm-link: {}", e)))?;

        // return the merged bitcode file
        Ok(merged_bc_path)
    }
}
