use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{anyhow, bail, Result};
use log::{debug, info, warn};

use libra_builder::ResolverLLVM;
use libra_engine::error::{EngineError, EngineResult};
use libra_engine::flow::fixedpoint::FlowFixedpoint;
use libra_engine::flow::shared::Context;
use libra_shared::config::{PARALLEL, PATH_STUDIO};
use libra_shared::dep::{DepState, Dependency, Resolver};
use libra_shared::git::GitRepo;

use crate::common::TestSuite;

/// Maximum number of fixedpoint optimization
static MAX_ROUNDS_OF_FIXEDPOINT_OPTIMIZATION: usize = 16;

static PATH_REPO: [&str; 2] = ["deps", "llvm-project"];
static PATH_WORKSPACE: [&str; 2] = ["testsuite", "external"];

/// Artifact path resolver for LLVM
pub struct ResolverLLVMInternal {
    resolver: ResolverLLVM,
    bin_lit: PathBuf,
    dir_test: PathBuf,
}

impl Resolver for ResolverLLVMInternal {
    fn construct(path: PathBuf) -> Self {
        let resolver = ResolverLLVM::construct(path);
        Self {
            bin_lit: resolver.path_build().join("bin").join("llvm-lit"),
            dir_test: resolver.path_build().join("test"),
            resolver,
        }
    }

    fn destruct(self) -> PathBuf {
        let Self { resolver, .. } = self;
        resolver.destruct()
    }

    fn seek() -> Result<(GitRepo, Self)> {
        DepState::<ResolverLLVMInternal, DepLLVMInternal>::new()?.into_source_and_artifact()
    }
}

/// Represent the llvm-project
pub struct DepLLVMInternal {}

impl Dependency<ResolverLLVMInternal> for DepLLVMInternal {
    fn repo_path_from_root() -> &'static [&'static str] {
        &PATH_REPO
    }

    fn list_build_options(_path_src: &Path, _path_config: &Path) -> Result<()> {
        bail!("attempting to setup LLVM internal test suite");
    }

    fn build(_path_src: &Path, _resolver: &ResolverLLVMInternal) -> Result<()> {
        bail!("attempting to build LLVM internal test suite");
    }
}

impl TestSuite<ResolverLLVMInternal> for DepLLVMInternal {
    fn run(
        repo: GitRepo,
        resolver: ResolverLLVMInternal,
        force: bool,
        filter: Vec<String>,
    ) -> Result<()> {
        // prepare the environment
        let mut workdir = PATH_STUDIO.to_path_buf();
        workdir.extend(PATH_WORKSPACE);
        if workdir.exists() {
            if !force {
                info!("Prior testing result exists");
                return Ok(());
            }
            fs::remove_dir_all(&workdir)?;
        }
        fs::create_dir_all(&workdir)?;

        // information collection
        let test_cases = Self::lit_test_discovery(&repo, &resolver)?;
        info!("Number of test cases discovered: {}", test_cases.len());

        // run the tests
        let ctxt = Context::new()?;
        if *PARALLEL && filter.is_empty() {
            todo!()
        } else {
            // serial execution will halt on first failure caused by potential bugs
            let mut results = vec![];
            for test in test_cases {
                // apply filter if necessary
                if !filter.is_empty() && !filter.contains(&test.name) {
                    continue;
                }

                // actual execution
                let output = test.run_libra(&ctxt, &workdir)?;

                // check errors to halt on first failure caused by potential bugs
                if let Some(Err(err)) = output.as_ref() {
                    match err {
                        EngineError::NotSupportedYet(_) | EngineError::CompilationError(_) => (),
                        EngineError::LLVMLoadingError(reason)
                        | EngineError::InvalidAssumption(reason)
                        | EngineError::InvariantViolation(reason) => {
                            bail!("potential bug: {}", reason);
                        }
                    }
                }
                results.push(output);
            }
        }

        Ok(())
    }
}

impl DepLLVMInternal {
    fn lit_test_discovery(
        repo: &GitRepo,
        resolver: &ResolverLLVMInternal,
    ) -> Result<Vec<BitcodeTestCase>> {
        // run discovery
        let output = Command::new(&resolver.bin_lit)
            .arg("--show-tests")
            .arg(&resolver.dir_test)
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
            let path_test = repo.path().join("llvm").join("test").join(name);
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
            // ignores machine code
            if name.starts_with("MC/") {
                continue;
            }
            // ignore deny-listed ones
            // TODO: they are not bitcode files, check them regularly
            if matches!(name, "Other/lit-globbing.ll" | "tools/llvm-ar/bitcode.ll") {
                continue;
            }

            // validate the test case
            Self::validate_ll_test_case(&path_test)
                .map_err(|e| anyhow!("invalid test case {}: {}", name, e))?;

            // add to worklist
            result.push(BitcodeTestCase {
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

struct BitcodeTestCase {
    name: String,
    path: PathBuf,
}

impl BitcodeTestCase {
    /// Run the test case through libra workflow (internal)
    pub fn run_libra(&self, ctxt: &Context, workdir: &Path) -> Result<Option<EngineResult<()>>> {
        let Self { name, path } = self;

        // report progress
        debug!("running test case: {}", name);

        // check if opt can verify the bitcode
        match ctxt.opt_verify(path) {
            Ok(_) => (),
            Err(e) => {
                warn!("unable to validate bitcode {} via opt: {}", name, e);
                return Ok(None);
            }
        }

        // prepare output directory
        let output_dir = workdir.join(name);
        fs::create_dir_all(&output_dir)?;

        // workflow
        let result = libra_workflow(ctxt, path, &output_dir);
        Ok(Some(result))
    }
}

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
