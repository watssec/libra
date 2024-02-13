use std::path::Path;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::common::AppConfig;
use crate::snippet;

/// Workflow configuration
#[derive(Serialize, Deserialize)]
pub struct Config {}

impl AppConfig for Config {
    fn app() -> &'static str {
        "pcre2"
    }

    fn build(&self, path_src: &Path, path_bin: &Path) -> Result<bool> {
        let mut rebuild = false;

        // prep
        rebuild = snippet::git_clone(
            path_src,
            "https://github.com/PCRE2Project/pcre2.git",
            rebuild,
        )?;

        // build
        snippet::build_via_autoconf(path_src, path_bin, Some(&[]), &[], rebuild)
    }
}
