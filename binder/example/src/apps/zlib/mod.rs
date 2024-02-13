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
        "zlib"
    }

    fn build(&self, path_src: &Path, path_bin: &Path) -> Result<()> {
        snippet::git_clone(path_src, "https://github.com/madler/zlib.git")?;
        snippet::build_via_autoconf(path_src, path_bin, None, &[])
    }
}
