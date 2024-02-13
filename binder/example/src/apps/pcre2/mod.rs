use std::path::Path;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::common::WorkflowConfig;
use crate::snippet;
use crate::snippet::mark_output_lib;
use crate::wllvm;

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

        // build
        let mut rebuild = false;
        rebuild = snippet::git_clone(
            &path_src,
            "https://github.com/PCRE2Project/pcre2.git",
            rebuild,
        )?;
        rebuild = snippet::build_via_autoconf(&path_src, &path_bin, Some(&[]), &[], rebuild)?;

        // check
        if rebuild {
            let path_lib_build = path_src.join(".libs");
            let path_lib_install = path_bin.join("lib");
            mark_output_lib("pcre2-8", &path_lib_install, &path_lib_build)?;
            mark_output_lib("pcre2-posix", &path_lib_install, &path_lib_build)?;
        }

        // merge
        wllvm::merge(&path_src, &path_bin)?;

        Ok(())
    }
}
