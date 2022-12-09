use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Result};
use fs_extra::dir::CopyOptions;
use log::{debug, error, info};
use rayon::prelude::{IntoParallelIterator, ParallelIterator};
use structopt::StructOpt;
use tempfile::tempdir;
use walkdir::WalkDir;

use libra_engine::{analyze, EngineError};
use libra_shared::config::{PATH_ROOT, PATH_STUDIO};
use libra_shared::logging;

#[derive(StructOpt)]
#[structopt(
    name = "libra-testsuite",
    about = "The testsuite executor for LIBRA",
    rename_all = "kebab-case"
)]
struct Args {
    /// Studio directory
    #[structopt(short, long)]
    studio: Option<PathBuf>,

    /// Verbosity
    #[structopt(short, long)]
    verbose: bool,

    /// LLVM test-suite path
    #[structopt(short, long)]
    path_llvm_test_suite: Option<PathBuf>,

    /// Partial identifier of the test
    #[structopt(short, long)]
    include: Option<String>,

    /// Partial identifier of the test
    #[structopt(short, long)]
    exclude: Option<String>,

    /// Depth of fixedpoint optimization
    #[structopt(short, long, default_value = "4")]
    depth: usize,

    /// Keep the workflow artifacts in the studio
    #[structopt(short, long)]
    keep: bool,

    /// Output the results
    #[structopt(short, long)]
    output: Option<PathBuf>,
}

fn main() -> Result<()> {
    let args = Args::from_args();
    let Args {
        studio,
        verbose,
        path_llvm_test_suite,
        include,
        exclude,
        depth,
        keep,
        output,
    } = args;
    let studio = studio.as_ref().unwrap_or(&PATH_STUDIO);

    // setup logging
    logging::setup(verbose)?;

    // known bad cases
    // TODO: should eliminate this list
    let do_not_test: BTreeSet<_> = [
        // custom bitwidth attributes
        "SingleSource/UnitTests/Integer",
        // vector support is poor anyway
        "SingleSource/UnitTests/Vector",
    ]
    .into_iter()
    .collect();

    // collect test cases
    let path_llvm_test_suite =
        path_llvm_test_suite.unwrap_or_else(|| PATH_ROOT.join("deps").join("llvm-test-suite"));
    let test_cases = collect_test_cases(
        &path_llvm_test_suite,
        include.as_deref(),
        exclude.as_deref(),
        &do_not_test,
    )?;
    let total_num = test_cases.len();
    info!("total number of tests: {}", total_num);

    // run the tests sequentially if in keep (i.e., development) mode
    if keep {
        // prepare the artifact keeping path
        let path_artifact = studio.join("testing");
        if path_artifact.exists() {
            fs::remove_dir_all(&path_artifact)?;
        }

        // run over test cases one by one
        for TestCase { name, inputs } in test_cases {
            debug!("running: {}", name);

            match run_test_case(depth, inputs, Some(&path_artifact)) {
                TestResult::Pass(_) | TestResult::Unsupported | TestResult::Uncompilable => (),
                TestResult::Fail(err) => {
                    error!("{}", err);
                    bail!("unexpected analysis error");
                }
            }
        }

        info!("all tests complete gracefully");
        return Ok(());
    }

    // run the tests in parallel
    let results: BTreeMap<_, _> = test_cases
        .into_par_iter()
        .map(|TestCase { name, inputs }| (name, run_test_case(depth, inputs, None)))
        .collect();
    assert_eq!(results.len(), total_num);

    // split the results
    let mut result_pass = vec![];
    let mut result_fail = vec![];
    let mut result_unsupported = vec![];
    let mut result_uncompilable = vec![];

    for (name, result) in results {
        match result {
            TestResult::Pass(length) => result_pass.push((name, length)),
            TestResult::Unsupported => result_unsupported.push(name),
            TestResult::Uncompilable => result_uncompilable.push(name),
            TestResult::Fail(_) => result_fail.push(name),
        }
    }

    info!("passed: {}", result_pass.len());
    info!("failed: {}", result_fail.len());
    info!("unsupported: {}", result_unsupported.len());
    info!("uncompilable: {}", result_uncompilable.len());

    match output {
        None => (),
        Some(path) => {
            // write passed results
            let mut content = vec![];
            for (name, rounds) in result_pass {
                content.push(format!("{}:{}", name, rounds));
            }
            let path_pass = path.with_extension("pass");
            fs::write(&path_pass, content.join("\n"))?;

            // write failed resultgs
            content.clear();
            for name in result_fail {
                content.push(name);
            }
            let path_fail = path.with_extension("fail");
            fs::write(&path_fail, content.join("\n"))?;
        }
    }

    // done with the testing
    Ok(())
}

struct TestCase {
    name: String,
    inputs: Vec<PathBuf>,
}

fn collect_test_cases(
    path_llvm_test_suite: &Path,
    include: Option<&str>,
    exclude: Option<&str>,
    do_not_test: &BTreeSet<&str>,
) -> Result<Vec<TestCase>> {
    let mut tests = vec![];
    for entry in WalkDir::new(path_llvm_test_suite.join("SingleSource")) {
        let path = entry?.into_path();

        // filter non-source files
        let is_source_c = path.extension().map_or(false, |ext| ext == "c");
        if !is_source_c {
            continue;
        }
        // TODO: handle C++ cases

        // filter the test
        let should_include = include.map_or(true, |pattern| {
            path.as_os_str().to_string_lossy().contains(pattern)
        });
        if !should_include {
            continue;
        }
        let should_exclude = exclude.map_or(false, |pattern| {
            path.as_os_str().to_string_lossy().contains(pattern)
        });
        if should_exclude {
            continue;
        }

        // grab the name
        let name = path
            .strip_prefix(path_llvm_test_suite)
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();

        // deny those bad cases
        if do_not_test.iter().any(|e| name.contains(e)) {
            continue;
        }

        // register the test case
        tests.push(TestCase {
            name,
            inputs: vec![path],
        });
    }

    // TODO: collect multi-sources
    Ok(tests)
}

enum TestResult {
    Pass(usize),
    Unsupported,
    Uncompilable,
    Fail(EngineError),
}

fn run_test_case(depth: usize, inputs: Vec<PathBuf>, keep: Option<&Path>) -> TestResult {
    let temp = tempdir().expect("unable to create a temporary directory");
    match analyze(Some(depth), vec![], inputs, temp.path().to_path_buf()) {
        Ok(trace) => TestResult::Pass(trace.len()),
        Err(EngineError::NotSupportedYet(_)) => TestResult::Unsupported,
        Err(EngineError::CompilationError(_)) => TestResult::Uncompilable,
        Err(err) => {
            if let Some(path_artifact) = keep {
                fs::create_dir(path_artifact).expect("unable to create artifact-keeping directory");
                let options = CopyOptions {
                    content_only: true,
                    ..Default::default()
                };
                fs_extra::dir::copy(temp.path(), path_artifact, &options)
                    .expect("unable to copy artifact");
            }
            TestResult::Fail(err)
        }
    }
}
