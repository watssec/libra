use std::env;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Result};
use llvm_ir::Module as LLVMModule;

use crate::error::{EngineError, EngineResult};
use crate::ir;

pub struct Workflow {
    // llvm binaries
    bin_opt: PathBuf,
    bin_clang: PathBuf,
    bin_llvm_link: PathBuf,
    bin_llvm_dis: PathBuf,
    /// Source file
    inputs: Vec<PathBuf>,
    /// Workspace for the analysis
    output: PathBuf,
}

impl Workflow {
    pub fn new(inputs: Vec<PathBuf>, output: PathBuf) -> Self {
        let pkg_llvm = Path::new(env!("LIBRA_CONST_LLVM_ARTIFACT"));
        Self {
            bin_opt: pkg_llvm.join("bin").join("opt"),
            bin_clang: pkg_llvm.join("bin").join("clang"),
            bin_llvm_link: pkg_llvm.join("bin").join("llvm-link"),
            bin_llvm_dis: pkg_llvm.join("bin").join("llvm-dis"),
            inputs,
            output,
        }
    }
}

impl Workflow {
    fn get_init_bc_path(&self, index: usize) -> PathBuf {
        self.output.join(format!("init-{}.bc", index))
    }
    fn get_merged_bc_path(&self) -> PathBuf {
        self.output.join("merged.bc")
    }

    pub fn execute(&self) -> EngineResult<()> {
        // compilation
        let mut init_bc_files = vec![];
        for (i, src) in self.inputs.iter().enumerate() {
            let bc_path = self.get_init_bc_path(i);
            self.run_clang(
                src,
                &bc_path,
                [
                    // output llvm bitcode
                    "-c",
                    "-emit-llvm",
                    // attack debug symbol
                    "-g",
                    // targeting the C language
                    "--language",
                    "c",
                    // do not include standard items
                    "-nostdinc",
                    "-nostdlib",
                    // feature selection
                    "-std=c17",
                ],
            )
            .map_err(|e| EngineError::CompilationError(format!("Error during clang: {}", e)))?;
            self.disassemble(&bc_path)
                .map_err(|e| EngineError::CompilationError(format!("Error during disas: {}", e)))?;
            init_bc_files.push(bc_path);
        }

        // linking
        let path_refs: Vec<_> = init_bc_files.iter().map(|p| p.as_path()).collect();
        let merged_bc_path = self.get_merged_bc_path();
        self.run_llvm_link(&path_refs, &merged_bc_path, ["--internalize"])
            .map_err(|e| EngineError::CompilationError(format!("Error during llvm-link: {}", e)))?;
        self.run_opt(&merged_bc_path, None, ["--verify"])
            .map_err(|e| {
                EngineError::CompilationError(format!("Error during opt --verify: {}", e))
            })?;
        self.disassemble(&merged_bc_path)
            .map_err(|e| EngineError::CompilationError(format!("Error during disas: {}", e)))?;

        // baseline loading
        let llvm_module =
            LLVMModule::from_bc_path(&merged_bc_path).map_err(EngineError::LLVMLoadingError)?;
        ir::bridge::convert(&llvm_module)?;

        // TODO: optimization until a fixedpoint

        // TODO: analysis
        Ok(())
    }

    fn disassemble(&self, input: &Path) -> Result<()> {
        let output = input.with_extension("ll");
        self.run_llvm_dis(input, &output)
    }
}

/// Command execution internals
impl Workflow {
    fn run(mut cmd: Command) -> Result<()> {
        let status = cmd.status()?;
        if !status.success() {
            bail!(
                "Command failed with status {}: {} {}",
                status,
                cmd.get_program().to_str().unwrap(),
                cmd.get_args()
                    .map(|arg| arg.to_str().unwrap())
                    .collect::<Vec<_>>()
                    .join(" ")
            );
        }
        Ok(())
    }

    fn run_opt<I, S>(&self, input: &Path, output: Option<&Path>, args: I) -> Result<()>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let mut cmd = Command::new(&self.bin_opt);
        cmd.args(args)
            .arg("-o")
            .arg(output.unwrap_or_else(|| Path::new("/dev/null")));
        cmd.arg(input);
        Self::run(cmd)
    }

    fn run_clang<I, S>(&self, input: &Path, output: &Path, args: I) -> Result<()>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let mut cmd = Command::new(&self.bin_clang);
        cmd.args(args).arg("-o").arg(output).arg(input);
        Self::run(cmd)
    }

    fn run_llvm_link<I, S>(&self, input: &[&Path], output: &Path, args: I) -> Result<()>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let mut cmd = Command::new(&self.bin_llvm_link);
        cmd.args(args).arg("-o").arg(output).args(input);
        Self::run(cmd)
    }

    fn run_llvm_dis(&self, input: &Path, output: &Path) -> Result<()> {
        let mut cmd = Command::new(&self.bin_llvm_dis);
        cmd.arg("-o").arg(output).arg(input);
        Self::run(cmd)
    }
}
