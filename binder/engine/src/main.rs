use std::fs;
use std::path::PathBuf;
use std::str::FromStr;

use anyhow::{bail, Result};
use libra_engine::flow::build_simple::FlowBuildSimple;
use libra_engine::flow::fixedpoint::FlowFixedpoint;
use log::info;
use structopt::StructOpt;
use tempfile::tempdir;

use libra_engine::flow::shared::Context;
use libra_shared::config::PATH_STUDIO;
use libra_shared::logging;

#[derive(StructOpt)]
#[structopt(
    name = "libra-engine",
    about = "The main execution engine for LIBRA",
    rename_all = "kebab-case"
)]
struct Args {
    /// Studio directory
    #[structopt(short, long)]
    studio: Option<PathBuf>,

    /// Verbosity
    #[structopt(short, long)]
    verbose: bool,

    /// Keep the workflow artifacts in the studio
    #[structopt(short, long)]
    keep: bool,

    /// Actions
    #[structopt(short, long)]
    actions: Vec<Action>,

    /// Source code files
    #[structopt(required = true)]
    inputs: Vec<PathBuf>,

    /// Extra flags to be passed to clang
    #[structopt(short, long)]
    flags: Vec<String>,

    /// Limit the depth of fixedpoint optimization
    #[structopt(short, long)]
    depth: Option<usize>,
}

#[derive(StructOpt)]
enum Action {
    /// Build the source code
    Build,
    /// Run fixedpoint optimization
    Fixedpoint,
}

impl FromStr for Action {
    type Err = &'static str;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let action = match s {
            "build" => Self::Build,
            "fixedpoint" => Self::Fixedpoint,
            _ => return Err("invalid action"),
        };
        Ok(action)
    }
}

fn main() -> Result<()> {
    let args = Args::from_args();
    let Args {
        studio,
        verbose,
        keep,
        mut actions,
        inputs,
        flags,
        depth,
    } = args;
    let studio = studio.as_ref().unwrap_or(&PATH_STUDIO);

    // setup logging
    logging::setup(verbose)?;

    // decide on the workspace
    let (temp, output) = if keep {
        let path = studio.join("libra");
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

    // run the workflow
    let ctxt = Context::new();

    // phase 1: see if anything to build
    let path_base_bitcode = match actions.iter().position(|a| matches!(a, Action::Build)) {
        None => {
            if inputs.len() != 1 {
                bail!("expecting one and only one input if not building from sources");
            }
            inputs.into_iter().next().unwrap()
        }
        Some(index) => {
            let path_output = match actions.remove(index) {
                Action::Build => {
                    FlowBuildSimple::new(&ctxt, inputs, output.clone(), flags).execute()?
                }
                _ => unreachable!(),
            };
            info!(
                "Bitcode generated at {}",
                path_output
                    .into_os_string()
                    .to_str()
                    .unwrap_or("<non-ascii-path>")
            );

            // ensure there is no more build actions
            if actions.iter().any(|a| matches!(a, Action::Build)) {
                bail!("only one build action is allowed");
            }
        }
    };

    // phase 2: any optimizations to run
    let _ir = match actions.iter().position(|a| matches!(a, Action::Fixedpoint)) {
        None => ctxt.load(&path_base_bitcode)?,
        Some(index) => match actions.remove(index) {
            Action::Fixedpoint => {
                let trace =
                    FlowFixedpoint::new(&ctxt, path_base_bitcode, output, depth).execute()?;
                if trace.is_empty() {
                    bail!("fixedpoint optimization leaves no modules in trace");
                }
                info!("Number of fixedpoint optimization rounds: {}", trace.len());
                trace.into_iter().rev().next().unwrap()
            }
            _ => unreachable!(),
        },
    };

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
