use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Result};
use fs_extra::dir::CopyOptions;
use log::{debug, error, info};
use structopt::StructOpt;
use tempfile::tempdir;
use walkdir::WalkDir;

use libra_engine::{analyze, EngineError};
use libra_shared::config::PATH_STUDIO;
use libra_shared::logging;

// TODO: get this from env!
static PATH_LLVM_TEST_SUITE: &str = "/home/mengxu/Research/llvm-test-suite";

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
    filter: Option<String>,

    /// Keep the workflow artifacts in the studio
    #[structopt(short, long)]
    keep: bool,
}

fn main() -> Result<()> {
    let args = Args::from_args();
    let Args {
        studio,
        verbose,
        path_llvm_test_suite,
        filter,
        keep,
    } = args;
    let studio = studio.as_ref().unwrap_or(&PATH_STUDIO);

    // setup logging
    logging::setup(verbose)?;

    // known bad cases
    // TODO: should eliminate this list
    let do_not_test: BTreeSet<_> = [
        // custom bitwidth attributes
        "SingleSource/UnitTests/Integer",
    ]
    .into_iter()
    .collect();

    // collect test cases
    let path_llvm_test_suite =
        path_llvm_test_suite.unwrap_or_else(|| PathBuf::from(PATH_LLVM_TEST_SUITE));
    let test_cases = collect_test_cases(&path_llvm_test_suite, filter.as_deref(), &do_not_test)?;
    let total_num = test_cases.len();
    info!("number of tests: {}", total_num);

    // run the tests one by one
    let mut result_pass = 0;
    let mut result_unsupported = 0;
    for TestCase { name, inputs } in test_cases {
        debug!("running: {}", name);
        let temp = tempdir().expect("unable to create a temporary directory");
        match analyze(
            vec![format!("-I{}/stdlib-mock", env!("CARGO_MANIFEST_DIR"))],
            inputs,
            temp.path().to_path_buf(),
        ) {
            Ok(_) => {
                result_pass += 1;
            }
            Err(EngineError::NotSupportedYet(_)) => {
                result_unsupported += 1;
            }
            Err(err) => {
                error!("{}", err);
                // save the result if requested
                if keep {
                    let path_artifact = studio.join("testing");
                    if path_artifact.exists() {
                        fs::remove_dir_all(&path_artifact)?;
                    }
                    fs::create_dir(&path_artifact)?;
                    let options = CopyOptions {
                        content_only: true,
                        ..Default::default()
                    };
                    fs_extra::dir::copy(temp.path(), &path_artifact, &options)?;

                    // shortcut the testing in debugging mode
                    bail!("unexpected analysis error");
                }
            }
        };
    }

    info!("passed: {}", result_pass);
    info!("unsupported: {}", result_unsupported);
    info!("error: {}", total_num - result_pass - result_unsupported);
    Ok(())
}

struct TestCase {
    name: String,
    inputs: Vec<PathBuf>,
}

fn collect_test_cases(
    path_llvm_test_suite: &Path,
    filter: Option<&str>,
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
        let ignored = filter.map_or(false, |pattern| {
            path.as_os_str().to_string_lossy().contains(pattern)
        });
        if ignored {
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
