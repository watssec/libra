use std::fs;
use std::path::Path;
use std::process::Command;

use libra_engine::flow::shared::Context;
use libra_shared::dep::Dependency;

use anyhow::{anyhow, Result};

#[cfg(target_os = "macos")]
use std::os::unix;

#[cfg(target_os = "macos")]
use tempfile::tempdir;

#[cfg(target_os = "macos")]
use libra_shared::config::{UNAME_HARDWARE, UNAME_PLATFORM};

static PATH_REPO: [&str; 2] = ["deps", "llvm-test-suite"];

/// Represent the llvm-test-suite
pub struct DepLLVMTestSuite {}

impl DepLLVMTestSuite {
    fn cmake_profile(path_src: &Path) -> Result<String> {
        path_src
            .join("cmake")
            .join("caches")
            .join("Debug.cmake")
            .into_os_string()
            .into_string()
            .map_err(|_| anyhow!("non-ascii path"))
    }
}

impl Dependency for DepLLVMTestSuite {
    fn repo_path_from_root() -> &'static [&'static str] {
        &PATH_REPO
    }

    fn list_build_options(path_src: &Path, path_build: &Path) -> Result<()> {
        let ctxt = Context::new();

        // dump cmake options
        let mut cmd = Command::new("cmake");
        cmd.arg("-LAH")
            .arg(format!(
                "-DCMAKE_C_COMPILER={}",
                ctxt.path_llvm(["bin", "clang"])?
            ))
            .arg(format!("-C{}", Self::cmake_profile(path_src)?))
            .arg(path_src);
        cmd.current_dir(path_build);
        let status = cmd.status()?;
        if !status.success() {
            return Err(anyhow!("Configure failed"));
        }

        // done
        Ok(())
    }

    fn build(path_src: &Path, path_build: &Path, _artifact: &Path) -> Result<()> {
        let ctxt = Context::new();

        // llvm configuration
        let mut cmd = Command::new("cmake");
        cmd.arg("-G")
            .arg("Ninja")
            .arg(format!(
                "-DCMAKE_C_COMPILER={}",
                ctxt.path_llvm(["bin", "clang"])?
            ))
            .arg(format!("-C{}", Self::cmake_profile(path_src)?))
            .arg("-DCMAKE_EXPORT_COMPILE_COMMANDS=ON");

        // platform-specific configuration
        #[cfg(target_os = "macos")]
        let tmp_sysroot = match (UNAME_PLATFORM.as_str(), UNAME_HARDWARE.as_str()) {
            ("Darwin", "arm64") => {
                // fake a sysroot directory for the tests
                let sysroot = tempdir()?;
                let sysroot_usr = sysroot.path().join("usr");
                fs::create_dir(&sysroot_usr)?;
                unix::fs::symlink(
                    ctxt.path_llvm(["include", "c++", "v1"])?,
                    sysroot_usr.join("include"),
                )?;
                unix::fs::symlink(ctxt.path_llvm(["lib"])?, sysroot_usr.join("lib"))?;
                cmd.arg(format!(
                    "-DCMAKE_OSX_SYSROOT={}",
                    sysroot
                        .path()
                        .to_str()
                        .ok_or_else(|| anyhow!("non-ascii sysroot path"))?
                ));
                sysroot
            }
            _ => {
                panic!("other macos platforms not supported yet");
            }
        };

        // done with the configuration
        cmd.arg(path_src);
        cmd.current_dir(path_build);
        let status = cmd.status()?;
        if !status.success() {
            return Err(anyhow!("Configure failed"));
        }

        // build
        let mut cmd = Command::new("cmake");
        cmd.arg("--build").arg(path_build);
        let status = cmd.status()?;
        if !status.success() {
            return Err(anyhow!("Build failed"));
        }

        #[cfg(target_os = "macos")]
        tmp_sysroot.close()?;

        // done
        Ok(())
    }
}
