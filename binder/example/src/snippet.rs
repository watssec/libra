use std::fs;
use std::path::Path;
use std::process::Command;

use anyhow::{anyhow, bail, Result};
use log::debug;

use crate::common::CLANG_WRAP;

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

/// Svn clone
pub fn svn_clone(path_src: &Path, repo: &str, mut rebuild: bool) -> Result<bool> {
    if rebuild || !path_src.exists() {
        let mut cmd = Command::new("svn");
        cmd.arg("checkout").arg(repo).arg(path_src);
        if !cmd.status()?.success() {
            bail!("unable to svn checkout {}", repo);
        }
        rebuild = true;
    } else {
        debug!("skipped: svn checkout {}", repo);
    }
    Ok(rebuild)
}

/// Build with autoconf process
pub fn build_via_autoconf(
    path_src: &Path,
    path_bin: &Path,
    skip_autogen: bool,
    configure_args: &[&str],
    mut rebuild: bool,
) -> Result<bool> {
    // autogen.sh
    if !skip_autogen {
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
        cmd.arg(format!(
            "--prefix={}",
            path_bin.to_str().ok_or_else(|| anyhow!("non-ascii path"))?
        ))
        .arg("--disable-silent-rules")
        .args(configure_args)
        .env("CC", CLANG_WRAP.as_str())
        .current_dir(path_src);
        if !cmd.status()?.success() {
            bail!("unable to configure");
        }
        rebuild = true;
    } else {
        debug!("skipped: configure")
    }

    // make
    if rebuild || !path_bin.exists() {
        let mut cmd = Command::new("make");
        cmd.current_dir(path_src);
        if !cmd.status()?.success() {
            bail!("unable to make");
        }

        let mut cmd = Command::new("make");
        cmd.arg("install").current_dir(path_src);
        if !cmd.status()?.success() {
            bail!("unable to make install");
        }
        rebuild = true;
    } else {
        debug!("skipped: make install")
    }

    Ok(rebuild)
}
