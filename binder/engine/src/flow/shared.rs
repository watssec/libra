use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{anyhow, bail, Result};
use libra_builder::{artifact_for_pass, ResolverLLVM};
use libra_shared::dep::Resolver;

use crate::error::{EngineError, EngineResult};
use crate::ir::{adapter, bridge};

/// Context for all workflow
pub struct Context {
    /// Path to the llvm installation base
    pkg_llvm: PathBuf,
    /// Path to the clang compiler
    bin_clang: PathBuf,
    /// Path to the llvm-link tool
    bin_llvm_link: PathBuf,
    /// Path to the llvm-dis tool
    bin_llvm_dis: PathBuf,
    /// Path to the opt tool
    bin_opt: PathBuf,
    /// Path to the libra pass
    lib_pass: PathBuf,
}

impl Context {
    pub fn new() -> Result<Self> {
        let (_, resolver_llvm) = ResolverLLVM::seek()?;
        let lib_pass = artifact_for_pass()?;
        let pkg_llvm = resolver_llvm.path_install().to_path_buf();

        Ok(Self {
            bin_clang: pkg_llvm.join("bin").join("clang"),
            bin_llvm_link: pkg_llvm.join("bin").join("llvm-link"),
            bin_llvm_dis: pkg_llvm.join("bin").join("llvm-dis"),
            bin_opt: pkg_llvm.join("bin").join("opt"),
            pkg_llvm,
            lib_pass: lib_pass.to_path_buf(),
        })
    }

    pub fn path_llvm<I, S>(&self, segments: I) -> Result<String>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<Path>,
    {
        let mut path = self.pkg_llvm.to_path_buf();
        path.extend(segments);
        path.into_os_string()
            .into_string()
            .map_err(|_| anyhow!("non-ascii llvm path"))
    }

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

    fn run_clang<I, S>(&self, input: &Path, output: &Path, args: I) -> Result<()>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let mut cmd = Command::new(&self.bin_clang);
        cmd.args(args).arg("-o").arg(output).arg(input);
        Self::run(cmd)
    }

    pub fn compile_to_bitcode<I, S>(&self, input: &Path, output: &Path, args: I) -> Result<()>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let mut flags = vec![OsString::from("-c"), OsString::from("-emit-llvm")];
        flags.extend(args.into_iter().map(|i| i.as_ref().to_os_string()));
        self.run_clang(input, output, flags)
    }

    pub fn link_bitcode(&self, input: &[&Path], output: &Path) -> Result<()> {
        let mut cmd = Command::new(&self.bin_llvm_link);
        cmd.arg("--internalize").arg("-o").arg(output).args(input);
        Self::run(cmd)
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

    /// Disassemble the bitcode file into readable format
    pub fn disassemble(&self, input: &Path, output: &Path) -> Result<()> {
        let mut cmd = Command::new(&self.bin_llvm_dis);
        cmd.arg("-o").arg(output).arg(input);
        Self::run(cmd)
    }

    /// Disassemble the bitcode file into readable format in the same directory
    pub fn disassemble_in_place(&self, input: &Path) -> Result<()> {
        let output = input.with_extension("ll");
        self.disassemble(input, &output)
    }

    /// Verify the consistency of the bitcode file
    pub fn opt_verify(&self, input: &Path) -> Result<()> {
        self.run_opt(input, None, ["-passes=verify"])
    }

    /// Run a specified opt pipeline
    pub fn opt_pipeline(&self, input: &Path, output: &Path, pipeline: &str) -> Result<()> {
        self.run_opt(input, Some(output), [format!("--passes={}", pipeline)])
    }

    /// Serialize a bitcode file to JSON
    fn serialize(&self, input: &Path, output: &Path) -> Result<()> {
        let lib_pass = self
            .lib_pass
            .to_str()
            .ok_or_else(|| anyhow!("non-ascii path"))?;
        self.run_opt(
            input,
            None,
            [
                &format!("-load-pass-plugin={}", lib_pass),
                "-passes=Libra",
                &format!("--libra-output={}", output.to_str().unwrap()),
            ],
        )
    }

    /// Deserialize the JSON file to a module
    fn deserialize(input: &Path) -> EngineResult<bridge::module::Module> {
        let content = fs::read_to_string(input)
            .map_err(|e| EngineError::LLVMLoadingError(format!("Corrupted JSON file: {}", e)))?;
        let module_adapted: adapter::module::Module =
            serde_json::from_str(&content).map_err(|e| {
                EngineError::LLVMLoadingError(format!("Error during deserialization: {}", e))
            })?;
        let module_bridge = bridge::module::Module::convert(&module_adapted)?;
        Ok(module_bridge)
    }

    /// Serialize a bitcode file to JSON and then load it as a module
    pub fn load(&self, input: &Path) -> EngineResult<bridge::module::Module> {
        let output = input.with_extension("json");
        self.serialize(input, &output).map_err(|e| {
            EngineError::LLVMLoadingError(format!("unable to serialize the bitcode file: {}", e))
        })?;
        Self::deserialize(&output)
    }
}
