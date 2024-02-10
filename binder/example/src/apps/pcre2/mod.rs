use std::path::Path;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::common::WorkflowConfig;
use crate::snippet;

/// Workflow configuration
#[derive(Serialize, Deserialize)]
pub struct Config {}

impl WorkflowConfig for Config {
    fn app() -> &'static str {
        "pcre2"
    }

    fn run(self, workdir: &Path) -> Result<()> {
        let path_src = workdir.join("src");
        let path_bin = workdir.join("bin");

        let mut rebuild = false;
        rebuild = snippet::git_clone(
            &path_src,
            "https://github.com/PCRE2Project/pcre2.git",
            rebuild,
        )?;
        snippet::build_via_autoconf(&path_src, &path_bin, Some(&[]), &[], rebuild)?;

        Ok(())
    }
}
