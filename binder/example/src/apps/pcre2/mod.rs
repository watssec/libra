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
        "pcre2"
    }

    fn run(self, workdir: &Path) -> Result<()> {
        // fetch source code exists
        let path_src = workdir.join("src");
        if !path_src.exists() {
            let mut cmd = Command::new("git");
            cmd.arg("clone")
                .arg("--depth=1")
                .arg("https://github.com/PCRE2Project/pcre2.git")
                .arg(&path_src);
            if !cmd.status()?.success() {
                bail!("unable to clone source repository");
            }
        } else {
            debug!("skipped: git clone <main>");
        }

        // configure
        let path_configure = path_src.join("configure");
        if !path_configure.exists() {
            let mut cmd = Command::new("./autogen.sh");
            cmd.current_dir(&path_src);
            if !cmd.status()?.success() {
                bail!("unable to autogen.sh");
            }
        } else {
            debug!("skipped: autogen.sh")
        }

        Ok(())
    }
}
