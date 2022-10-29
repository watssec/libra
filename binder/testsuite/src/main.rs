use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use log::info;
use structopt::StructOpt;
use tempfile::tempdir;
use walkdir::WalkDir;

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

    // decide on the workspace
    let (temp, output) = if keep {
        let path = studio.join("testing");
        if path.exists() {
            fs::remove_dir_all(&path)?;
        }
        fs::create_dir_all(&path)?;
        (None, path)
    } else {
        let dir = tempdir()?;
        let path = dir.path().to_path_buf();
        (Some(dir), path)
    };

    // collect test cases
    let path_llvm_test_suite =
        path_llvm_test_suite.unwrap_or_else(|| PathBuf::from(PATH_LLVM_TEST_SUITE));
    let test_cases = collect_test_cases(&path_llvm_test_suite, filter.as_deref())?;
    info!("number of tests: {}", test_cases.len());

    // drop temp dir explicitly
    match temp {
        None => (),
        Some(dir) => {
            dir.close()?;
        }
    };

    // done with everything
    Ok(())
}

fn collect_test_cases(
    path_llvm_test_suite: &Path,
    filter: Option<&str>,
) -> Result<Vec<Vec<PathBuf>>> {
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

        // register the test case
        tests.push(vec![path]);
    }

    // TODO: collect multi-sources
    Ok(tests)
}
