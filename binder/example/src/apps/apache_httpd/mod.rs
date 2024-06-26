use std::path::Path;
use std::process::Command;

use anyhow::{anyhow, bail, Result};
use serde::{Deserialize, Serialize};

use crate::common::AppConfig;
use crate::snippet;
use crate::workflow::retrieve_workflow;

/// Workflow configuration
#[derive(Serialize, Deserialize)]
pub struct Config {}

impl AppConfig for Config {
    fn app() -> &'static str {
        "apache_httpd"
    }

    fn build(&self, path_src: &Path, path_bin: &Path) -> Result<()> {
        // grab dependencies
        let (dep_pcre2, _) = retrieve_workflow::<crate::apps::pcre2::Config>("default")?;

        snippet::git_clone(path_src, "https://github.com/apache/httpd.git")?;
        snippet::svn_clone(
            &path_src.join("srclib").join("apr"),
            "https://svn.apache.org/repos/asf/apr/apr/trunk/",
        )?;
        snippet::svn_clone(
            &path_src.join("srclib").join("apr-util"),
            "https://svn.apache.org/repos/asf/apr/apr-util/trunk/",
        )?;

        // special configuration step for apache httpd
        let mut cmd = Command::new("./buildconf");
        cmd.current_dir(path_src);
        if !cmd.status()?.success() {
            bail!("unable to buildconf");
        }

        // resume normal autoconf procedure
        snippet::build_via_autoconf(
            path_src,
            path_bin,
            None,
            &[
                "--with-included-apr",
                &format!(
                    "--with-pcre={}",
                    dep_pcre2
                        .join("bin")
                        .into_os_string()
                        .into_string()
                        .map_err(|_| anyhow!("non-ascii path"))?
                ),
            ],
        )
    }
}
