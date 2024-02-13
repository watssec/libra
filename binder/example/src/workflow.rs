use std::collections::BTreeMap;
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};
use std::{env, fs};

use anyhow::{anyhow, bail, Result};
use lazy_static::lazy_static;
use log::info;
use serde::{Deserialize, Serialize};

use libra_engine::flow::fixedpoint::FlowFixedpoint;
use libra_engine::flow::shared::Context;
use libra_shared::config::PATH_STUDIO;

use crate::common::{derive_bitcode_path, AppConfig};
use crate::proxy::LIBMARK_EXTENSION;
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

/// Type of artifact
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ArtifactKind {
    Lib,
    Bin,
}

/// Entrypoint for analysis
#[derive(Serialize, Deserialize)]
struct Entrypoint {
    kind: ArtifactKind,
    name: String,
    function: String,
}

/// Stages of the workflow
enum Stage {
    Build,
    Check,
    Merge,
    Analyze,
}

impl Stage {
    fn mark(&self) -> &str {
        match self {
            Self::Build => "mark.build",
            Self::Check => "mark.check",
            Self::Merge => "mark.merge",
            Self::Analyze => "mark.analyze",
        }
    }

    /// set the stage mark
    pub fn set_mark(self, workdir: &Path) -> Result<()> {
        OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(workdir.join(self.mark()))?;
        Ok(())
    }

    /// get the stage mark
    pub fn get_mark(self, workdir: &Path) -> bool {
        workdir.join(self.mark()).exists()
    }
}

/// Workflow for an app
#[derive(Serialize, Deserialize)]
#[serde(bound = "T: AppConfig")]
pub struct Workflow<T: AppConfig> {
    // config
    config: T,
    // build
    libs: BTreeMap<String, Artifact>,
    bins: BTreeMap<String, Artifact>,
    entry: Entrypoint,
    // analysis
    fixedpoint: Option<usize>,
}

impl<T: AppConfig> Workflow<T> {
    /// Check build artifact
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

    /// Analyze the artifact based on entry point
    fn analyze(&self, path_src: &Path, path_bin: &Path, path_wks: &Path) -> Result<()> {
        // derive the bitcode file
        let artifact = match &self.entry.kind {
            ArtifactKind::Lib => match self.libs.get(&self.entry.name) {
                None => bail!("unable to find entry target in libs: {}", self.entry.name),
                Some(artifact) => path_bin
                    .join(&artifact.item_in_bin)
                    .join(format!("lib{}{}", self.entry.name, LIBMARK_EXTENSION))
                    .canonicalize()?,
            },
            ArtifactKind::Bin => match self.bins.get(&self.entry.name) {
                None => bail!("unable to find entry target in bins: {}", self.entry.name),
                Some(artifact) => path_src
                    .join(&artifact.item_in_src)
                    .join(&self.entry.name)
                    .canonicalize()?,
            },
        };
        if !artifact.exists() {
            bail!(
                "original artifact does not exist: {}",
                artifact.to_string_lossy()
            );
        }

        let path_base_bitcode = derive_bitcode_path(artifact);
        if !path_base_bitcode.exists() {
            bail!(
                "artifact bitcode does not exist: {}",
                path_base_bitcode.to_string_lossy()
            );
        }

        // prepare for analysis
        let ctxt = Context::new()?;
        fs::create_dir_all(path_wks)?;

        // fixedpoint optimization (if applicable)
        let trace = FlowFixedpoint::new(
            &ctxt,
            path_base_bitcode,
            path_wks.to_path_buf(),
            self.fixedpoint,
        )
        .execute()?;

        if trace.is_empty() {
            bail!("fixedpoint optimization leaves no modules in trace");
        }
        info!("Number of fixedpoint optimization rounds: {}", trace.len());
        trace.into_iter().next_back().unwrap();

        // done
        Ok(())
    }

    /// Execute the profile
    pub fn run(&self, workdir: &Path) -> Result<()> {
        let path_src = workdir.join("src");
        let path_bin = workdir.join("bin");
        let path_wks = workdir.join("wks");

        // obtain the bitcode
        if !Stage::Build.get_mark(workdir) {
            T::build(&self.config, &path_src, &path_bin)?;
            Stage::Build.set_mark(workdir)?;
        }
        if !Stage::Check.get_mark(workdir) {
            self.check(&path_src, &path_bin)?;
            Stage::Check.set_mark(workdir)?;
        }
        if !Stage::Merge.get_mark(workdir) {
            wllvm::merge(&path_src, &path_bin)?;
            Stage::Merge.set_mark(workdir)?;
        }

        // run the analysis
        if !Stage::Analyze.get_mark(workdir) {
            self.analyze(&path_src, &path_bin, &path_wks)?;
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

    // execute the workflows one by one
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
