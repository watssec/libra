use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::{env, fs};

use anyhow::{anyhow, bail, Result};
use lazy_static::lazy_static;
use libra_shared::config::PATH_STUDIO;
use log::info;
use serde::{Deserialize, Serialize};

use crate::common::AppConfig;
use crate::{snippet, wllvm};

lazy_static! {
    static ref FORCE: bool = matches!(env::var("FORCE"), Ok(val) if val == "1");
}

/// Details for a library artifact
#[derive(Serialize, Deserialize)]
struct Artifact {
    item_in_src: String,
    item_in_bin: String,
}

/// Workflow for an app
#[derive(Serialize, Deserialize)]
#[serde(bound = "T: AppConfig")]
pub struct Workflow<T: AppConfig> {
    libs: BTreeMap<String, Artifact>,
    bins: BTreeMap<String, Artifact>,
    config: T,
}

impl<T: AppConfig> Workflow<T> {
    /// Check process
    fn check(&self, path_src: &Path, path_bin: &Path) -> Result<()> {
        for (name, details) in &self.libs {
            snippet::mark_artifact_lib(
                name,
                path_bin.join(&details.item_in_bin),
                path_src.join(&details.item_in_src),
            )?;
        }
        for (name, details) in &self.bins {
            snippet::check_artifact_bin(
                name,
                path_bin.join(&details.item_in_bin),
                path_src.join(&details.item_in_src),
            )?;
        }
        Ok(())
    }

    /// Execute the profile
    pub fn run(&self, workdir: &Path) -> Result<()> {
        let path_src = workdir.join("src");
        let path_bin = workdir.join("bin");

        let rebuild = T::build(&self.config, &path_src, &path_bin)?;
        if rebuild {
            self.check(&path_src, &path_bin)?;
            wllvm::merge(&path_src, &path_bin)?;
        }

        Ok(())
    }
}

/// Probe for workflows available
fn probe_workflows<T: AppConfig>() -> Result<Vec<(String, PathBuf, Workflow<T>)>> {
    let app = T::app();
    let mut workflows = vec![];

    let base = Path::new(env!("CARGO_MANIFEST_DIR"));
    let config_dir = base.join("src").join("apps").join(app).join("configs");
    for item in config_dir.read_dir()? {
        let item = item?;
        let path = item.path();
        if path.extension().map_or(false, |e| e == "json") {
            let content = fs::read_to_string(&path)?;
            let workflow: Workflow<T> = serde_json::from_str(&content)?;

            let filename = item
                .file_name()
                .into_string()
                .map_err(|_| anyhow!("non-ascii string"))?;
            let name = filename
                .strip_suffix(".json")
                .expect("strip the .json suffix")
                .to_string();
            let workdir = PATH_STUDIO.join("example").join(app).join(&name);

            workflows.push((name, workdir, workflow));
        }
    }

    Ok(workflows)
}

/// Run the workflows based on defined config files
pub fn execute<T: AppConfig>() -> Result<()> {
    let app = T::app();
    let workflows = probe_workflows::<T>()?;

    // execute the configs one by one
    for (name, workdir, workflow) in workflows {
        info!("Processing '{}' under config '{}'", app, name);

        // prepare the work directory
        if workdir.exists() && *FORCE {
            fs::remove_dir_all(&workdir)?;
        }
        if !workdir.exists() {
            fs::create_dir_all(&workdir)?;
        }

        // execute it
        workflow.run(&workdir)?;
    }
    Ok(())
}

/// Retrieve a particular workflow
pub fn retrieve_workflow<T: AppConfig>(target: &str) -> Result<(PathBuf, Workflow<T>)> {
    let configs = probe_workflows::<T>()?;
    for (name, workdir, config) in configs {
        if target == name.as_str() {
            return Ok((workdir, config));
        }
    }
    bail!("no such workflow '{}' for app '{}'", target, T::app());
}
