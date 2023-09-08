use std::path::{Path, PathBuf};
use std::{env, fs};

use anyhow::{bail, Result};
use libra_engine::flow::shared::Context;

use libra_shared::compile_db::ClangCommand;

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

    /// Run the test case through libra workflow
    pub fn run_libra(&self, ctxt: &Context, workdir: &Path) -> Result<Option<()>> {
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

        // prepare output directory
        let output_dir = workdir.join(name);
        fs::create_dir_all(&output_dir)?;

        // temporarily change directory
        let cursor = env::current_dir()?;
        env::set_current_dir(&command.workdir)?;

        // compile
        let output = output_dir.join("init.bc");
        ctxt.compile_to_bitcode(Path::new(input), &output, command.gen_args_for_libra())?;

        // clean-up
        env::set_current_dir(cursor)?;
        Ok(Some(()))
    }
}
