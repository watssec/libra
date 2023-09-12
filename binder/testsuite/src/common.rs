use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use anyhow::Result;
use log::error;
use serde::{Deserialize, Serialize};

use libra_engine::error::{EngineError, EngineResult};
use libra_engine::flow::shared::Context;
use libra_shared::dep::Resolver;
use libra_shared::git::GitRepo;

/// A trait that marks a test suite
pub trait TestSuite<R: Resolver> {
    /// Run the test suite
    fn run(repo: GitRepo, resolver: R, force: bool, filter: Vec<String>) -> Result<()>;
}

/// A trait that marks a test case
pub trait TestCase {
    /// Run the test case through libra workflow (internal)
    fn run_libra(
        &self,
        ctxt: &Context,
        workdir: &Path,
    ) -> Result<(String, Option<EngineResult<()>>)>;
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
