use std::path::Path;
use std::process::Command;

use libra_engine::flow::shared::Context;
use libra_shared::dep::Dependency;

use anyhow::{anyhow, Result};

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
            .arg(format!("-DCMAKE_C_COMPILER={}", ctxt.path_clang()?))
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
            .arg(format!("-DCMAKE_C_COMPILER={}", ctxt.path_clang()?))
            .arg(format!("-C{}", Self::cmake_profile(path_src)?))
            .arg("-DCMAKE_EXPORT_COMPILE_COMMANDS=ON")
            .arg(path_src);
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

        // done
        Ok(())
    }
}
