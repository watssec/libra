use std::path::Path;
use std::process::Command;

use anyhow::{anyhow, bail, Result};
use log::debug;
use serde::{Deserialize, Serialize};

use crate::common::{retrieve_config, WorkflowConfig};
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
        let path_bin = workdir.join("bin");

        // grab dependencies
        let (dep_pcre2, _) = retrieve_config::<crate::apps::pcre2::Config>("default")?;

        let mut rebuild = false;
        rebuild = snippet::git_clone(&path_src, "https://github.com/apache/httpd.git", rebuild)?;
        rebuild = snippet::svn_clone(
            &path_src.join("srclib/apr"),
            "https://svn.apache.org/repos/asf/apr/apr/trunk/",
            rebuild,
        )?;

        // special configuration step for apache httpd
        let path_configure = path_src.join("configure");
        if rebuild || !path_configure.exists() {
            let mut cmd = Command::new("./buildconf");
            cmd.current_dir(&path_src);
            if !cmd.status()?.success() {
                bail!("unable to buildconf");
            }
            rebuild = true;
        } else {
            debug!("skipped: buildconf")
        }

        // resume normal autoconf procedure
        snippet::build_via_autoconf(
            &path_src,
            &path_bin,
            true,
            &[
                "--with-included-apr",
                &format!(
                    "--with-pcre={}",
                    dep_pcre2
                        .to_str()
                        .ok_or_else(|| anyhow!("non-ascii path"))?
                ),
            ],
            rebuild,
        )?;

        Ok(())
    }
}
