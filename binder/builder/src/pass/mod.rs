use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{anyhow, bail, Result};

use libra_shared::config::PATH_ROOT;
use libra_shared::dep::{DepState, Dependency};

use crate::deps::llvm::ArtifactForPass;

/// Represent the Oracle dependency
pub struct DepOracle {}

impl Dependency for DepOracle {
    fn name() -> &'static str {
        "oracle"
    }

    fn tweak(_path_wks: &Path) -> Result<()> {
        bail!("not supported");
    }

    fn build(path_wks: &Path) -> Result<()> {
        // prepare paths and deps
        let path_src = PATH_ROOT.join("oracle");
        let artifact_llvm = ArtifactForPass::seek()?;

        // configure
        let mut cmd = Command::new("cmake");
        cmd.arg("-G")
            .arg("Ninja")
            .arg(format!(
                "-DCFG_LLVM_INSTALL_DIR={}",
                artifact_llvm
                    .path_install
                    .to_str()
                    .ok_or_else(|| anyhow!("non-ascii path"))?
            ))
            .arg("-DCMAKE_BUILD_TYPE=Debug")
            .arg(path_src)
            .current_dir(path_wks);
        let status = cmd.status()?;
        if !status.success() {
            bail!("Configure failed with status {}", status);
        }

        // build
        let mut cmd = Command::new("cmake");
        cmd.arg("--build").arg(path_wks);
        let status = cmd.status()?;
        if !status.success() {
            bail!("Build failed with status {}", status);
        }

        // done
        Ok(())
    }
}

/// Artifact to be consumed by the analysis engine
#[non_exhaustive]
pub struct Artifact {
    pub path_lib: PathBuf,
}

impl Artifact {
    pub fn seek() -> Result<Self> {
        let path_wks = DepState::<DepOracle>::new()?.artifact()?;
        Ok(Self {
            path_lib: path_wks.join("Libra").join("libLibra.so"),
        })
    }
}
