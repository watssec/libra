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
        "apache_httpd"
    }

    fn run(self, workdir: &Path) -> Result<()> {
        let path_src = workdir.join("src");
        // let path_bin = workdir.join("bin");

        let mut rebuild = false;
        rebuild = snippet::git_clone(&path_src, "https://github.com/apache/httpd.git", rebuild)?;
        snippet::svn_clone(
            &path_src.join("srclib/apr"),
            "https://svn.apache.org/repos/asf/apr/apr/trunk/",
            rebuild,
        )?;

        Ok(())
    }
}
