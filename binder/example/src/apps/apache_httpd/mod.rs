use std::path::Path;
use std::process::Command;

use anyhow::{bail, Result};
use log::debug;
use serde::{Deserialize, Serialize};

use crate::common::WorkflowConfig;

/// Workflow configuration
#[derive(Serialize, Deserialize)]
pub struct Config {}

impl WorkflowConfig for Config {
    fn app() -> &'static str {
        "apache_httpd"
    }

    fn run(self, workdir: &Path) -> Result<()> {
        // fetch source code exists
        let path_src = workdir.join("src");
        if !path_src.exists() {
            let mut cmd = Command::new("git");
            cmd.arg("clone")
                .arg("--depth=1")
                .arg("https://github.com/apache/httpd.git")
                .arg(&path_src);
            if !cmd.status()?.success() {
                bail!("unable to clone source repository");
            }
        } else {
            debug!("source code repository ready");
        }

        // fetch dependencies
        let path_dep_apr = path_src.join("srclib/apr");
        if !path_dep_apr.exists() {
            let mut cmd = Command::new("svn");
            cmd.arg("checkout")
                .arg("https://svn.apache.org/repos/asf/apr/apr/trunk/")
                .arg("srclib/apr")
                .current_dir(&path_src);
            if !cmd.status()?.success() {
                bail!("unable to retrieve APR source code");
            }
        } else {
            debug!("APR source ready");
        }

        Ok(())
    }
}
