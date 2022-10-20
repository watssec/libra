use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{anyhow, Result};

/// Represents a Git-based repository
pub struct GitRepo {
    path: PathBuf,
    commit: String,
}

impl GitRepo {
    /// Create a representation of the repo
    pub fn new(path: PathBuf, version: Option<&str>) -> Result<Self> {
        let mut cmd = Command::new("git");
        cmd.arg("rev-list");
        cmd.arg("-n").arg("1").arg(version.unwrap_or("HEAD"));
        cmd.current_dir(&path);
        let output = cmd.output()?;
        if !output.status.success() {
            return Err(anyhow!("Commit probing failed"));
        }
        let commit = String::from_utf8(output.stdout)?.trim().to_string();
        Ok(Self { path, commit })
    }

    /// Retrieve the commit hash of this version
    pub fn commit(&self) -> &str {
        &self.commit
    }

    /// Checkout the repo into a new directory
    pub fn checkout(&mut self, path_src: &Path) -> Result<()> {
        if path_src.exists() {
            return Err(anyhow!("Checkout path already exists: {:?}", path_src));
        }

        // clone
        let mut cmd = Command::new("git");
        cmd.arg("clone")
            .arg(
                self.path
                    .as_os_str()
                    .to_str()
                    .ok_or_else(|| anyhow!("Invalid path: {:?}", path_src))?,
            )
            .arg(path_src);
        let status = cmd.status()?;
        if !status.success() {
            return Err(anyhow!("Clone failed"));
        }

        // checkout
        let mut cmd = Command::new("git");
        cmd.arg("checkout");
        cmd.arg(&self.commit);
        cmd.current_dir(&path_src);
        let status = cmd.status()?;
        if !status.success() {
            return Err(anyhow!("Checkout failed"));
        }

        // done
        Ok(())
    }
}
