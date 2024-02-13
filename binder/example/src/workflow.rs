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
struct ArtifactLib {
    item_in_src: Vec<String>,
    item_in_bin: Vec<String>,
}

/// Details for a binary artifact
#[derive(Serialize, Deserialize)]
struct ArtifactBin {
    item_in_src: Vec<String>,
}

/// Workflow for an app
#[derive(Serialize, Deserialize)]
#[serde(bound = "T: AppConfig")]
pub struct Workflow<T: AppConfig> {
    libs: BTreeMap<String, ArtifactLib>,
    bins: BTreeMap<String, ArtifactBin>,
    config: T,
}

impl<T: AppConfig> Workflow<T> {
    /// Check process
    fn check(&self, path_src: &Path, path_bin: &Path) -> Result<()> {
        // libraries
        for (name, details) in &self.libs {
            let mut path_install = path_bin.to_path_buf();
            path_install.extend(details.item_in_bin.iter());

            let mut path_build = path_src.to_path_buf();
            path_build.extend(details.item_in_src.iter());

            snippet::mark_artifact_lib(name, &path_install, &path_build)?;
        }

        // binaries
        for (name, details) in &self.bins {
            let mut path_build = path_src.to_path_buf();
            path_build.extend(details.item_in_src.iter());

            snippet::check_artifact_bin(name, &path_build)?;
        }

        // done
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

/// Probe for configs available
fn probe_configs<T: AppConfig>() -> Result<Vec<(String, PathBuf, Workflow<T>)>> {
    let app = T::app();
    let mut configs = vec![];

    let base = Path::new(env!("CARGO_MANIFEST_DIR"));
    let config_dir = base.join("src").join("apps").join(app).join("configs");
    for item in config_dir.read_dir()? {
        let item = item?;
        let path = item.path();
        if path.extension().map_or(false, |e| e == "json") {
            let content = fs::read_to_string(&path)?;
            let config: Workflow<T> = serde_json::from_str(&content)?;

            let filename = item
                .file_name()
                .into_string()
                .map_err(|_| anyhow!("non-ascii string"))?;
            let name = filename
                .strip_suffix(".json")
                .expect("strip the .json suffix")
                .to_string();
            let workdir = PATH_STUDIO.join("example").join(app).join(&name);

            configs.push((name, workdir, config));
        }
    }

    Ok(configs)
}

/// Run the workflows based on defined config files
pub fn execute<T: AppConfig>() -> Result<()> {
    let app = T::app();
    let configs = probe_configs::<T>()?;

    // execute the configs one by one
    for (name, workdir, config) in configs {
        info!("Processing '{}' under config '{}'", app, name);

        // prepare the work directory
        if workdir.exists() && *FORCE {
            fs::remove_dir_all(&workdir)?;
        }
        if !workdir.exists() {
            fs::create_dir_all(&workdir)?;
        }

        // execute it
        config.run(&workdir)?;
    }
    Ok(())
}

/// Retrieve the config
pub fn retrieve_config<T: AppConfig>(target: &str) -> Result<(PathBuf, Workflow<T>)> {
    let configs = probe_configs::<T>()?;
    for (name, workdir, config) in configs {
        if target == name.as_str() {
            return Ok((workdir, config));
        }
    }
    bail!("no such config '{}' for app '{}'", target, T::app());
}
