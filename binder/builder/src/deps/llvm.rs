use std::path::Path;
use std::process::Command;

use anyhow::{anyhow, Result};

#[cfg(target_os = "macos")]
use libra_shared::config::{UNAME_HARDWARE, UNAME_PLATFORM, UNAME_RELEASE, XCODE_SDK_PATH};

use crate::deps::common::Dependency;

// path constants
static PATH_REPO: [&str; 2] = ["deps", "llvm-project"];

/// Represent the LLVM deps
pub struct DepLLVM {}

impl Dependency for DepLLVM {
    fn repo_path_from_root() -> &'static [&'static str] {
        &PATH_REPO
    }

    fn list_build_options(path_src: &Path, path_build: &Path) -> Result<()> {
        // dump cmake options
        let mut cmd = Command::new("cmake");
        cmd.arg("-LAH")
            .arg("-DCMAKE_BUILD_TYPE=Debug")
            .arg(path_src.join("llvm"));
        cmd.current_dir(path_build);
        let status = cmd.status()?;
        if !status.success() {
            return Err(anyhow!("Configure failed"));
        }

        // done
        Ok(())
    }

    fn build(path_src: &Path, path_build: &Path, artifact: &Path) -> Result<()> {
        // llvm configuration
        let mut cmd = Command::new("cmake");
        cmd.arg("-G")
            .arg("Ninja")
            .arg(format!(
                "-DLLVM_ENABLE_PROJECTS={}",
                ["clang", "clang-tools-extra", "lld", "lldb", "polly"].join(";")
            ))
            .arg(format!(
                "-DLLVM_ENABLE_RUNTIMES={}",
                ["compiler-rt", "libc", "libcxx"].join(";")
            ))
            .arg("-DLLVM_ENABLE_RTTI=On")
            .arg("-DBUILD_SHARED_LIBS=On")
            .arg("-DCMAKE_BUILD_TYPE=Debug");

        // platform-specific configuration
        #[cfg(target_os = "macos")]
        match (UNAME_PLATFORM.as_str(), UNAME_HARDWARE.as_str()) {
            ("Darwin", "arm64") => {
                cmd.arg("-DCMAKE_OSX_ARCHITECTURES=arm64")
                    .arg("-DLLVM_TARGETS_TO_BUILD=AArch64")
                    .arg(format!(
                        "-DLLVM_DEFAULT_TARGET_TRIPLE=aarch64-apple-darwin{}",
                        UNAME_RELEASE.as_str()
                    ))
                    .arg(format!("-DDEFAULT_SYSROOT={}", XCODE_SDK_PATH.as_str()));
            }
            _ => (),
        }

        // done with the configuration
        cmd.arg(path_src.join("llvm"));
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

        // install
        let mut cmd = Command::new("cmake");
        cmd.arg("--install")
            .arg(path_build)
            .arg("--prefix")
            .arg(artifact);
        let status = cmd.status()?;
        if !status.success() {
            return Err(anyhow!("Install failed"));
        }

        // done
        Ok(())
    }
}
