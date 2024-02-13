use std::fs;
use std::os::unix::fs::symlink;
use std::path::Path;
use std::process::Command;

use anyhow::{anyhow, bail, Result};

use crate::common::{CLANG_CPP_WRAP, CLANG_WRAP};
use crate::proxy::{COMMAND_EXTENSION, LIBMARK_EXTENSION};

/// Git clone
pub fn git_clone(path_src: &Path, repo: &str) -> Result<()> {
    if path_src.exists() {
        bail!("directory already exists:{}", path_src.to_string_lossy());
    }

    let mut cmd = Command::new("git");
    cmd.arg("clone").arg("--depth=1").arg(repo).arg(path_src);
    if !cmd.status()?.success() {
        bail!("unable to git clone {}", repo);
    }
    Ok(())
}

/// Svn clone
pub fn svn_clone(path_src: &Path, repo: &str) -> Result<()> {
    if path_src.exists() {
        bail!("directory already exists:{}", path_src.to_string_lossy());
    }

    let mut cmd = Command::new("svn");
    cmd.arg("co").arg(repo).arg(path_src);
    if !cmd.status()?.success() {
        bail!("unable to svn clone {}", repo);
    }
    Ok(())
}

/// Build with autoconf process
pub fn build_via_autoconf(
    path_src: &Path,
    path_bin: &Path,
    args_autogen: Option<&[&str]>,
    args_configure: &[&str],
) -> Result<()> {
    // autogen.sh
    match args_autogen {
        None => { /* skip autogen */ }
        Some(args) => {
            let mut cmd = Command::new("./autogen.sh");
            cmd.args(args);
            cmd.current_dir(path_src);
            if !cmd.status()?.success() {
                bail!("unable to autogen.sh");
            }
        }
    }

    // configure
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

    // make
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

    // done
    Ok(())
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
pub fn check_artifact_bin<P: AsRef<Path>, Q: AsRef<Path>>(
    name: &str,
    path_install: P,
    path_build: Q,
) -> Result<()> {
    // check in install
    if !path_install.as_ref().join(name).exists() {
        bail!("binary artifact does not exist in bin {}", name);
    }

    // find the target
    let command_file = format!("{}{}", name, COMMAND_EXTENSION);

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
        bail!("binary artifact does not exist in src {}", name);
    }

    // done
    Ok(())
}
