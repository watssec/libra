use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::{bail, Result};
use log::{error, info};
use rayon::iter::IntoParallelIterator;
use rayon::iter::ParallelIterator;
use serde::{Deserialize, Serialize};

use libra_engine::error::{EngineError, EngineResult};
use libra_engine::flow::shared::Context;
use libra_shared::config::{CONTINUE, PARALLEL, PATH_STUDIO};
use libra_shared::dep::Resolver;
use libra_shared::git::GitRepo;

/// Controls whether we need to halt the parallel execution
static HALT_PARALLEL_EXECUTION: AtomicBool = AtomicBool::new(false);

/// A trait that marks a test case
pub trait TestCase: Send {
    /// Get the name of the test case
    fn name(&self) -> &str;

    /// Run the test case through libra workflow
    fn run_libra(
        &self,
        ctxt: &Context,
        workdir: &Path,
    ) -> Result<(String, Option<EngineResult<()>>)>;
}

/// A trait that marks a test suite
pub trait TestSuite<C: TestCase, R: Resolver> {
    /// Location of the workspace from the studio
    fn wks_path_from_studio() -> &'static [&'static str];

    /// Test case discovery
    fn discover_test_cases(repo: &GitRepo, resolver: &R) -> Result<Vec<C>>;

    /// Run the test suite
    fn run(repo: GitRepo, resolver: R, force: bool, filter: Vec<String>) -> Result<()> {
        // prepare the environment
        let mut workdir = PATH_STUDIO.to_path_buf();
        workdir.extend(Self::wks_path_from_studio());
        if workdir.exists() {
            if !force {
                info!("Prior testing result exists");
                return Ok(());
            }
            fs::remove_dir_all(&workdir)?;
        }
        fs::create_dir_all(&workdir)?;

        // information collection
        let test_cases = Self::discover_test_cases(&repo, &resolver)?;
        info!("Number of test cases discovered: {}", test_cases.len());

        // run the tests
        let ctxt = Context::new()?;
        let consolidated: Vec<_> = if *PARALLEL && filter.is_empty() {
            test_cases
                .into_par_iter()
                .map(|test| {
                    if HALT_PARALLEL_EXECUTION.load(Ordering::SeqCst) {
                        // not executing this one
                        return Ok((test.name().to_string(), None));
                    }
                    let (name, output) = test.run_libra(&ctxt, &workdir)?;
                    match shall_halt(&output) {
                        None => (),
                        Some(message) => {
                            if !*CONTINUE {
                                if HALT_PARALLEL_EXECUTION.swap(true, Ordering::SeqCst) {
                                    // not reporting this one
                                    return Ok((test.name().to_string(), None));
                                } else {
                                    // report this one and we have marked the execution to halt
                                    error!("potential bug: {}", message);
                                }
                            }
                        }
                    }
                    Ok((name, output))
                })
                .collect::<Result<_>>()?
        } else {
            let mut results = vec![];
            for test in test_cases {
                // apply filter if necessary
                if !filter.is_empty() && filter.iter().all(|v| v != test.name()) {
                    continue;
                }

                // actual execution
                let (name, output) = test.run_libra(&ctxt, &workdir)?;

                // check errors
                match shall_halt(&output) {
                    None => (),
                    Some(message) => {
                        error!("potential bug: {}", message);
                        if !*CONTINUE {
                            // halt on first failure caused by potential bugs
                            bail!("halting sequential execution for potential bugs");
                        }
                    }
                }
                results.push((name, output));
            }
            results
        };

        // summarize the result
        let summary = Summary::new(consolidated);
        summary.show();

        let path_summary = workdir.join("summary.json");
        summary.save(&path_summary)?;
        info!("Summary saved at: {}", path_summary.to_string_lossy());

        // done
        Ok(())
    }
}

/// A utility to check whether this error means a potential bug
fn shall_halt<T>(output: &Option<EngineResult<T>>) -> Option<&str> {
    match output.as_ref()?.as_ref().err()? {
        EngineError::NotSupportedYet(_) | EngineError::CompilationError(_) => None,
        EngineError::LLVMLoadingError(reason)
        | EngineError::InvalidAssumption(reason)
        | EngineError::InvariantViolation(reason) => Some(reason),
    }
}

/// A summary for the testing result
#[derive(Serialize, Deserialize)]
pub struct Summary {
    passed: Vec<String>,
    skipped: Vec<String>,
    failed_compile: Vec<String>,
    failed_loading: Vec<String>,
    failed_invariant: Vec<String>,
    failed_assumption: Vec<String>,
    failed_unsupported: BTreeMap<String, Vec<String>>,
}

impl Summary {
    pub fn new(consolidated: Vec<(String, Option<EngineResult<()>>)>) -> Self {
        let size = consolidated.len();

        // split the results
        let mut passed = vec![];
        let mut skipped = vec![];
        let mut failed_compile = vec![];
        let mut failed_loading = vec![];
        let mut failed_invariant = vec![];
        let mut failed_assumption = vec![];
        let mut failed_unsupported = BTreeMap::new();

        let mut name_set = BTreeSet::new();
        for (name, result) in consolidated {
            name_set.insert(name.clone());
            match result {
                None => skipped.push(name),
                Some(Ok(_)) => passed.push(name),
                Some(Err(err)) => match err {
                    // potential setup issue
                    EngineError::CompilationError(_) => {
                        failed_compile.push(name);
                    }
                    // known issues
                    EngineError::NotSupportedYet(reason) => {
                        failed_unsupported
                            .entry(reason)
                            .or_insert_with(Vec::new)
                            .push(name);
                    }
                    // potential bugs with the oracle
                    EngineError::LLVMLoadingError(_) => {
                        failed_loading.push(name);
                    }
                    // potential bugs with the backend
                    EngineError::InvariantViolation(_) => {
                        failed_invariant.push(name);
                    }
                    EngineError::InvalidAssumption(_) => {
                        failed_assumption.push(name);
                    }
                },
            }
        }

        // ensure consistency
        if name_set.len() != size {
            error!(
                "execution returns {} results but consolidated into {}",
                size,
                name_set.len()
            );
        }
        Self {
            passed,
            skipped,
            failed_compile,
            failed_loading,
            failed_invariant,
            failed_assumption,
            failed_unsupported: failed_unsupported
                .into_iter()
                .map(|(k, v)| (k.to_string(), v))
                .collect(),
        }
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    pub fn show(&self) {
        println!("passed: {}", self.passed.len());
        if !self.skipped.is_empty() {
            println!("skipped: {}", self.skipped.len());
        }
        if !self.failed_compile.is_empty() {
            println!("failed [compile]: {}", self.failed_compile.len());
        }
        if !self.failed_loading.is_empty() {
            println!("failed [loading]: {}", self.failed_loading.len());
        }
        if !self.failed_invariant.is_empty() {
            println!("failed [invariant]: {}", self.failed_invariant.len());
        }
        if !self.failed_assumption.is_empty() {
            println!("failed [assumption]: {}", self.failed_assumption.len());
        }
        println!(
            "unsupported: {}",
            self.failed_unsupported
                .values()
                .map(|v| v.len())
                .sum::<usize>()
        );
        for (category, tests) in &self.failed_unsupported {
            println!("  - {}: {}", category, tests.len());
        }
    }
}
