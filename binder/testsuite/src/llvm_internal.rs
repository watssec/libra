use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{anyhow, bail, Result};
use log::{debug, warn};

use libra_builder::deps::llvm::ArtifactLLVM;
use libra_engine::error::EngineResult;
use libra_engine::flow::fixedpoint::FlowFixedpoint;
use libra_engine::flow::shared::Context;
use libra_shared::dep::Dependency;

use crate::common::{TestCase, TestSuite};

/// Maximum number of fixedpoint optimization
static MAX_ROUNDS_OF_FIXEDPOINT_OPTIMIZATION: usize = 16;

/// Represent the llvm-project
pub struct DepLLVMInternal {}

impl Dependency for DepLLVMInternal {
    fn name() -> &'static str {
        "llvm-testsuite-internal"
    }

    fn tweak(_path_wks: &Path) -> Result<()> {
        bail!("attempting to tweat LLVM internal test suite");
    }

    fn build(_path_wks: &Path) -> Result<()> {
        let artifact_llvm = ArtifactLLVM::seek()?;

        // check
        let mut cmd = Command::new("cmake");
        cmd.arg("--build")
            .arg(&artifact_llvm.path_build)
            .arg("--target")
            .arg("stage2-check-llvm");
        let status = cmd.status()?;
        if !status.success() {
            bail!("Check failed with status {}", status);
        }

        // done
        Ok(())
    }
}

impl TestSuite<TestCaseInternal> for DepLLVMInternal {
    fn tag() -> &'static str {
        Self::name()
    }

    fn discover_test_cases() -> Result<Vec<TestCaseInternal>> {
        Self::lit_test_discovery()
    }
}

impl DepLLVMInternal {
    fn lit_test_discovery() -> Result<Vec<TestCaseInternal>> {
        // locate the paths and the lit tool
        let artifact_llvm = ArtifactLLVM::seek()?;
        let bin_lit = artifact_llvm
            .path_build_final_stage
            .join("bin")
            .join("llvm-lit");

        // run discovery
        let output = Command::new(bin_lit)
            .arg("--show-tests")
            .arg(artifact_llvm.path_build_final_stage.join("test"))
            .output()?;

        // sanity check the execution
        if !output.stderr.is_empty() {
            warn!(
                "stderr: {}",
                String::from_utf8(output.stderr)
                    .unwrap_or_else(|_| "<unable-to-parse>".to_string())
            );
        }
        if !output.status.success() {
            bail!("lit test discovery fails");
        }

        let content = String::from_utf8(output.stdout)?;
        let mut lines = content.lines();

        // skip first line
        if lines.next().map_or(true, |l| l != "-- Available Tests --") {
            bail!("invalid header line");
        }

        // parse the result
        let mut result = vec![];
        let mut test_name_exts = BTreeMap::new();
        for line in lines {
            let mut tokens = line.trim().split(" :: ");

            let ty = tokens.next().ok_or_else(|| anyhow!("expect test type"))?;
            if ty != "LLVM" {
                bail!("unexpected test type: {}", ty);
            }

            let name = tokens.next().ok_or_else(|| anyhow!("expect test name"))?;
            let path_test = artifact_llvm.path_src.join("llvm").join("test").join(name);
            if !path_test.exists() {
                bail!("test marker does not exist: {}", name);
            }

            // collect some statistics
            let ext = match path_test.extension() {
                None => String::new(),
                Some(e) => e
                    .to_str()
                    .ok_or_else(|| anyhow!("non-ascii path"))?
                    .to_string(),
            };
            let count = test_name_exts.entry(ext.clone()).or_insert(0_usize);
            *count += 1;

            // filter on llvm bitcode files (.ll) only
            if "ll" != ext {
                continue;
            }
            // ignore machine code
            if name.starts_with("MC/") {
                continue;
            }
            // ignore architecture-specific codegen
            if name.starts_with("CodeGen/") {
                continue;
            }
            // ignore deny-listed ones
            // TODO: they are not bitcode files, check them regularly
            if matches!(name, "Other/lit-globbing.ll" | "tools/llvm-ar/bitcode.ll") {
                continue;
            }
            // TODO: the following cases are ignored because we do not take `token` type
            if matches!(
                name,
                "Assembler/token.ll"
                    | "Bitcode/bcanalyzer-types.ll"
                    | "tools/llvm-reduce/reduce-instructions-token.ll"
                    | "tools/llvm-reduce/reduce-opcodes-call.ll"
            ) {
                continue;
            }
            // TODO: this case is explicitly ignored as an edge case
            //   In comment of the test case:
            //     "it would take a naive recursive implementation ~4 days"
            //   and our oracle is a naive recursive implementation
            if matches!(name, "tools/llvm-as/slow-ptrtoint.ll") {
                continue;
            }
            // TODO: this case is explicitly ignored as an edge case
            //   malformed landing pad found:
            //   landingpad { ptr, i32 } catch ptr inttoptr (i64 42 to ptr)
            if matches!(
                name,
                "Transforms/LoopStrengthReduce/X86/eh-insertion-point-2.ll"
            ) {
                continue;
            }

            // TODO: this case is explicitly ignored due to a potential bug with llvm
            //    Quoting the comment of the test case
            //      "There was optimization bug in ScalarEvolution,
            //       that causes too long compute time and stack overflow crash"
            //    It seems that ScalarEvolution might be okay but other passes are not
            if matches!(name, "Analysis/ScalarEvolution/avoid-assume-hang.ll") {
                continue;
            }

            // validate the test case
            Self::validate_ll_test_case(&path_test)
                .map_err(|e| anyhow!("invalid test case {}: {}", name, e))?;

            // add to worklist
            result.push(TestCaseInternal {
                name: name.to_string(),
                path: path_test,
            });
        }

        // show some stats in debug mode
        debug!("test case file extensions");
        for (ext, count) in test_name_exts {
            debug!("  - {}: {}", ext, count);
        }
        Ok(result)
    }

    fn validate_ll_test_case(path: &Path) -> Result<()> {
        let mut commands = vec![];
        let content = fs::read_to_string(path)?;
        for line in content.lines() {
            // skipping the prefixes
            let mut cur = line.trim();
            cur = match cur.strip_prefix(';') {
                None => {
                    // not a comment line for sure
                    continue;
                }
                Some(remaining) => remaining.trim(),
            };
            while let Some(remaining) = cur.strip_prefix(';') {
                cur = remaining.trim();
            }
            let cmd = match cur.strip_prefix("RUN:") {
                None => {
                    // not a run command
                    continue;
                }
                Some(cmd) => cmd.trim().to_string(),
            };
            commands.push(cmd);
        }

        // validity of
        if commands.is_empty() {
            bail!("no valid RUN command");
        }
        Ok(())
    }
}

pub struct TestCaseInternal {
    name: String,
    path: PathBuf,
}

impl TestCaseInternal {
    fn libra_workflow(ctxt: &Context, input: &Path, output: &Path) -> EngineResult<()> {
        // fixedpoint
        let flow_fp = FlowFixedpoint::new(
            ctxt,
            input.to_path_buf(),
            output.to_path_buf(),
            Some(MAX_ROUNDS_OF_FIXEDPOINT_OPTIMIZATION),
        );
        flow_fp.execute()?;
        Ok(())
    }
}

impl TestCase for TestCaseInternal {
    fn name(&self) -> &str {
        &self.name
    }

    fn run_libra(
        &self,
        ctxt: &Context,
        workdir: &Path,
    ) -> Result<(String, Option<EngineResult<()>>)> {
        let Self { name, path } = self;

        // report progress
        debug!("running test case: {}", name);

        // prepare output directory
        let output_dir = workdir.join(name);
        fs::create_dir_all(&output_dir)?;

        // check if llvm-as and opt likes the bitcode
        let path_bc_init = output_dir.join("init.bc");
        match ctxt
            .assemble(path, &path_bc_init)
            .and_then(|_| ctxt.opt_verify(&path_bc_init))
        {
            Ok(_) => (),
            Err(e) => {
                warn!("unable to validate bitcode {}: {}", name, e);
                return Ok((name.to_string(), None));
            }
        }

        // workflow
        let result = Self::libra_workflow(ctxt, &path_bc_init, &output_dir);
        Ok((name.to_string(), Some(result)))
    }
}
