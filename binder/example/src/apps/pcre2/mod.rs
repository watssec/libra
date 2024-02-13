use std::collections::BTreeMap;
use std::path::Path;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::common::{ArtifactBin, ArtifactLib, WorkflowConfig};
use crate::snippet;

/// Workflow configuration
#[derive(Serialize, Deserialize)]
pub struct Config {
    libs: BTreeMap<String, ArtifactLib>,
    bins: BTreeMap<String, ArtifactBin>,
}

impl WorkflowConfig for Config {
    fn app() -> &'static str {
        "pcre2"
    }

    fn artifact_libs(&self) -> impl Iterator<Item = (&str, &ArtifactLib)> {
        self.libs.iter().map(|(k, v)| (k.as_str(), v))
    }

    fn artifact_bins(&self) -> impl Iterator<Item = (&str, &ArtifactBin)> {
        self.bins.iter().map(|(k, v)| (k.as_str(), v))
    }

    fn build(&self, path_src: &Path, path_bin: &Path) -> Result<bool> {
        let mut rebuild = false;

        // prep
        rebuild = snippet::git_clone(
            &path_src,
            "https://github.com/PCRE2Project/pcre2.git",
            rebuild,
        )?;

        // build
        snippet::build_via_autoconf(&path_src, &path_bin, Some(&[]), &[], rebuild)
    }
}
