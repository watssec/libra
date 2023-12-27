use std::fs;
use std::path::Path;
use std::process::Command;

use anyhow::{anyhow, bail, Result};
use log::debug;

use libra_engine::flow::shared::Context;

/// Git clone
pub fn git_clone(path_src: &Path, repo: &str, mut rebuild: bool) -> Result<bool> {
    if rebuild || !path_src.exists() {
        let mut cmd = Command::new("git");
        cmd.arg("clone").arg("--depth=1").arg(repo).arg(path_src);
        if !cmd.status()?.success() {
            bail!("unable to git clone {}", repo);
        }
        rebuild = true;
    } else {
        debug!("skipped: git clone {}", repo);
    }
    Ok(rebuild)
}

/// Build with autoconf process
pub fn build_via_autoconf(
    path_src: &Path,
    path_bin: &Path,
    configure_args: &[&str],
    mut rebuild: bool,
) -> Result<bool> {
    // run the workflow
    let ctxt = Context::new()?;

    // autogen.sh
    let path_configure = path_src.join("configure");
    if rebuild || !path_configure.exists() {
        let mut cmd = Command::new("./autogen.sh");
        cmd.current_dir(path_src);
        if !cmd.status()?.success() {
            bail!("unable to autogen.sh");
        }
        rebuild = true;
    } else {
        debug!("skipped: autogen.sh")
    }

    // configure
    let path_makefile = path_src.join("Makefile");
    if rebuild || !path_makefile.exists() {
        // clean-up the installation directory
        if path_bin.exists() {
            fs::remove_dir_all(path_bin)?;
        }

        // re-generate the configure script
        let mut cmd = Command::new("./configure");
        cmd.env("CC", ctxt.path_llvm(["bin", "clang"])?)
            .arg(format!(
                "--prefix={}",
                path_bin.to_str().ok_or_else(|| anyhow!("non-ascii path"))?
            ))
            .arg("--disable-silent-rules")
            .args(configure_args)
            .current_dir(path_src);
        if !cmd.status()?.success() {
            bail!("unable to configure");
        }
        rebuild = true;
    } else {
        debug!("skipped: configure")
    }

    // make

    Ok(rebuild)
}
