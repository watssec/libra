use std::fs;
use std::os::unix::fs::symlink;
use std::path::Path;
use std::process::Command;

use anyhow::{anyhow, bail, Result};
use log::debug;

use crate::common::{CLANG_CPP_WRAP, CLANG_WRAP};
use crate::proxy::{COMMAND_EXTENSION, LIBMARK_EXTENSION};

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
        debug!("[build] skipped: git clone {}", repo);
    }
    Ok(rebuild)
}

/// Svn clone
pub fn svn_clone(path_src: &Path, repo: &str, mut rebuild: bool) -> Result<bool> {
    if rebuild || !path_src.exists() {
        let mut cmd = Command::new("svn");
        cmd.arg("co").arg(repo).arg(path_src);
        if !cmd.status()?.success() {
            bail!("unable to svn checkout {}", repo);
        }
        rebuild = true;
    } else {
        debug!("[build] skipped: svn checkout {}", repo);
    }
    Ok(rebuild)
}

/// Build with autoconf process
pub fn build_via_autoconf(
    path_src: &Path,
    path_bin: &Path,
    args_autogen: Option<&[&str]>,
    args_configure: &[&str],
    mut rebuild: bool,
) -> Result<bool> {
    // autogen.sh
    match args_autogen {
        None => { /* not applicable */ }
        Some(args) => {
            let path_configure = path_src.join("configure");
            if rebuild || !path_configure.exists() {
                debug!("[build] running autogen.sh");
                let mut cmd = Command::new("./autogen.sh");
                cmd.args(args);
                cmd.current_dir(path_src);
                if !cmd.status()?.success() {
                    bail!("unable to autogen.sh");
                }
                rebuild = true;
            } else {
                debug!("[build] skipped: autogen.sh");
            }
        }
    }

    // configure
    let path_makefile = path_src.join("Makefile");
    if rebuild || !path_makefile.exists() {
        debug!("[build] running configure");

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
        .args(args_configure)
        .env("CC", CLANG_WRAP.as_str())
        .env("CXX", CLANG_CPP_WRAP.as_str())
        .current_dir(path_src);
        if !cmd.status()?.success() {
            bail!("unable to configure");
        }
        rebuild = true;
    } else {
        debug!("[build] skipped: configure");
    }

    // make
    if rebuild || !path_bin.exists() {
        debug!("[build] running make");

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
        debug!("[build] skipped: make install");
    }

    // done
    Ok(rebuild)
}

/// Mark a library artifact
pub fn mark_artifact_lib<P: AsRef<Path>, Q: AsRef<Path>>(
    name: &str,
    path_install: P,
    path_build: Q,
) -> Result<()> {
    let prefix = format!("lib{}.", name);

    // get target path
    let mut target = None;
    for entry in fs::read_dir(path_build)? {
        let entry = entry?;
        let name = match entry.file_name().into_string() {
            Ok(n) => n,
            Err(_) => {
                // not even ascii path
                continue;
            }
        };
        if !name.starts_with(&prefix) {
            continue;
        }

        let original = match name.strip_suffix(COMMAND_EXTENSION) {
            None => {
                // does not bear a build instruction
                continue;
            }
            Some(base) => base,
        };

        // target found
        if target.is_some() {
            bail!("more than one library target for {}", name);
        }

        let mut path = entry.path();
        path.pop();
        target = Some(path.join(original));
    }
    let src = match target {
        None => bail!("no target to mark for library {}", name),
        Some(path) => path,
    };

    // create the symbolic link
    let dst = path_install
        .as_ref()
        .join(format!("lib{}{}", name, LIBMARK_EXTENSION));
    symlink(src, dst)?;

    // done
    Ok(())
}

/// Check that a binary artifact exists
pub fn check_artifact_bin<P: AsRef<Path>>(name: &str, path_build: P) -> Result<()> {
    let command_file = format!("{}{}", name, COMMAND_EXTENSION);

    // find the target
    let mut found = false;
    for entry in fs::read_dir(path_build)? {
        let entry = entry?;
        if entry
            .file_name()
            .into_string()
            .map_or(false, |n| n == command_file)
        {
            // target found
            if found {
                bail!("more than one binary target for {}", name);
            }
            found = true;
        }
    }
    if !found {
        bail!("binary artifact does not exist {}", name);
    }

    // done
    Ok(())
}
