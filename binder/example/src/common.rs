use std::path::{Path, PathBuf};
use std::{env, fs};

use anyhow::{anyhow, bail, Result};
use lazy_static::lazy_static;
use log::info;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use libra_shared::config::PATH_STUDIO;

use crate::{snippet, wllvm};

lazy_static! {
    static ref FORCE: bool = matches!(env::var("FORCE"), Ok(val) if val == "1");
}

lazy_static! {
    pub static ref CLANG_WRAP: String = {
        let mut target_dir = PathBuf::from(env!("LIBRA_TARGET_DIR"));
        target_dir.push("clang_wrap");
        target_dir
            .into_os_string()
            .into_string()
            .expect("ASCII path only")
    };
    pub static ref CLANG_CPP_WRAP: String = {
        let mut target_dir = PathBuf::from(env!("LIBRA_TARGET_DIR"));
        target_dir.push("clang_cpp_wrap");
        target_dir
            .into_os_string()
            .into_string()
            .expect("ASCII path only")
    };
}

/// Details for a library artifact
#[derive(Serialize, Deserialize)]
pub struct ArtifactLib {
    item_in_src: Vec<String>,
    item_in_bin: Vec<String>,
}

/// Details for a binary artifact
#[derive(Serialize, Deserialize)]
pub struct ArtifactBin {
    item_in_src: Vec<String>,
}

/// Common trait for workflow config
pub trait WorkflowConfig: Serialize + DeserializeOwned {
    /// Obtain the application name
    fn app() -> &'static str;

    /// List library artifacts
    fn artifact_libs(&self) -> impl Iterator<Item = (&str, &ArtifactLib)>;

    /// List binary artifacts
    fn artifact_bins(&self) -> impl Iterator<Item = (&str, &ArtifactBin)>;

    /// Build process
    fn build(&self, path_src: &Path, path_bin: &Path) -> Result<bool>;

    /// Check process
    fn check(&self, path_src: &Path, path_bin: &Path) -> Result<()> {
        // libraries
        for (name, details) in self.artifact_libs() {
            let mut path_install = path_bin.to_path_buf();
            path_install.extend(details.item_in_bin.iter());
            let mut path_build = path_src.to_path_buf();
            path_build.extend(details.item_in_src.iter());

            snippet::mark_artifact_lib(name, &path_install, &path_build)?;
        }

        // binaries
        for (name, details) in self.artifact_bins() {
            let mut path_build = path_src.to_path_buf();
            path_build.extend(details.item_in_src.iter());

            snippet::check_artifact_bin(name, &path_build)?;
        }

        // done
        Ok(())
    }

    /// Execute the profile
    fn run(self, workdir: &Path) -> Result<()> {
        let path_src = workdir.join("src");
        let path_bin = workdir.join("bin");

        let rebuild = self.build(&path_src, &path_bin)?;
        if rebuild {
            self.check(&path_src, &path_bin)?;
            wllvm::merge(&path_src, &path_bin)?;
        }

        Ok(())
    }
}

/// Run the workflows based on defined config files
fn probe_configs<T: WorkflowConfig>() -> Result<Vec<(String, PathBuf, T)>> {
    let app = T::app();
    let mut configs = vec![];

    let base = Path::new(env!("CARGO_MANIFEST_DIR"));
    let config_dir = base.join("src").join("apps").join(app).join("configs");
    for item in config_dir.read_dir()? {
        let item = item?;
        let path = item.path();
        if path.extension().map_or(false, |e| e == "json") {
            let content = fs::read_to_string(&path)?;
            let config: T = serde_json::from_str(&content)?;

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
pub fn execute<T: WorkflowConfig>() -> Result<()> {
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
pub fn retrieve_config<T: WorkflowConfig>(target: &str) -> Result<(PathBuf, T)> {
    let configs = probe_configs::<T>()?;
    for (name, workdir, config) in configs {
        if target == name.as_str() {
            return Ok((workdir, config));
        }
    }
    bail!("no such config '{}' for app '{}'", target, T::app());
}
