use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::anyhow;
use datatest_stable::{harness, Result};
use fs_extra::dir::CopyOptions;
use tempfile::tempdir;

use libra_engine::error::EngineResult;
use libra_engine::flow::build_simple::FlowBuildSimple;
use libra_engine::flow::fixedpoint::FlowFixedpoint;
use libra_engine::flow::shared::Context;
use libra_engine::ir::bridge;

#[derive(Copy, Clone)]
enum Verbosity {
    None,
    Normal,
    Verbose,
    Extensive,
}

fn workflow(
    ctxt: &Context,
    inputs: Vec<PathBuf>,
    output: PathBuf,
) -> EngineResult<Vec<bridge::module::Module>> {
    // build
    let flow_build = FlowBuildSimple::new(
        ctxt,
        inputs,
        output.clone(),
        vec![
            // do not include standard items
            "-nostdinc".into(),
            "-nostdlib".into(),
        ],
    );
    let merged_bc = flow_build.execute()?;

    // fixedpoint optimization
    let flow_fixedpoint = FlowFixedpoint::new(ctxt, merged_bc, output, None);
    flow_fixedpoint.execute()
}

fn run_test(path_output: &Path) -> Result<()> {
    // ready context
    let ctxt = Context::new()?;

    // config based on environment variable
    let keep = env::var("KEEP").map_or(false, |v| v == "1");
    let verbosity =
        env::var("LOG").map_or(Verbosity::None, |v| match v.parse::<usize>().unwrap() {
            0 => Verbosity::None,
            1 => Verbosity::Normal,
            2 => Verbosity::Verbose,
            _ => Verbosity::Extensive,
        });

    // load the expected result
    let expected = fs::read_to_string(path_output)
        .expect("unable to load content from the expected output file");

    // setup the directories
    let path_dir = path_output
        .parent()
        .expect("unable to locate the test case directory");

    let path_artifact = path_dir.join("testing");
    if path_artifact.exists() {
        fs::remove_dir_all(&path_artifact)?;
    }

    // collect source files
    let mut inputs = vec![];
    for item in fs::read_dir(path_dir).expect("unable to list the test case directory") {
        let item = item.unwrap();
        let name = item.file_name().into_string().unwrap();
        if name.ends_with(".c") {
            inputs.push(path_dir.join(name));
        }
    }

    // create output dir
    let temp = tempdir().expect("unable to create a temporary directory");
    let success = match workflow(&ctxt, inputs, temp.path().to_path_buf()) {
        Ok(trace) => {
            if expected.is_empty() {
                if matches!(verbosity, Verbosity::Verbose | Verbosity::Extensive) {
                    println!("Number of optimization rounds: {}", trace.len());
                }
                true
            } else {
                println!(
                    "Analysis succeeded while failure is expected:\n{}",
                    expected
                );
                false
            }
        }
        Err(err) => {
            let obtained = err.to_string();
            if expected.is_empty() {
                println!("Analysis failed while success is expected:\n{}", obtained);
                false
            } else if expected != obtained {
                println!(
                    "Error message mismatch:\n{}\n<- expected vs obtained ->\n{}",
                    expected, obtained
                );
                false
            } else {
                true
            }
        }
    };

    // save the workspace if on verbose mode or on failed test cases, if requested
    if matches!(verbosity, Verbosity::Extensive) || (keep && !success) {
        fs::create_dir(&path_artifact)?;
        // copy over the content
        let options = CopyOptions {
            content_only: true,
            ..Default::default()
        };
        fs_extra::dir::copy(temp.path(), &path_artifact, &options)?;
    }

    // clean-up
    temp.close()
        .expect("unable to clean-up the temporary directory");

    // report back
    if success {
        Ok(())
    } else {
        Err(anyhow!("result does not match with expectation").into())
    }
}

harness!(run_test, "tests", r"output");
