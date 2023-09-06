use std::fs;
use std::path::Path;
use std::process::Command;

use anyhow::{anyhow, Result};
use fs_extra::dir;

use libra_engine::flow::shared::Context;
use libra_shared::dep::Dependency;

static PATH_REPO: [&str; 2] = ["deps", "llvm-test-suite"];

fn baseline_cmake_options(path_src: &Path) -> Result<Vec<String>> {
    let ctxt = Context::new();
    let profile = path_src
        .join("cmake")
        .join("caches")
        .join("Debug.cmake")
        .into_os_string()
        .into_string()
        .map_err(|_| anyhow!("non-ascii path"))?;

    Ok(vec![
        format!("-DCMAKE_C_COMPILER={}", ctxt.path_llvm(["bin", "clang"])?),
        format!("-C{}", profile),
        format!(
            "-DTEST_SUITE_SUBDIRS={}",
            ["SingleSource", "MultiSource", "Bitcode"].join(";")
        ),
    ])
}

/// Represent the llvm-test-suite
pub struct DepLLVMTestSuite {}

impl Dependency for DepLLVMTestSuite {
    fn repo_path_from_root() -> &'static [&'static str] {
        &PATH_REPO
    }

    fn list_build_options(path_src: &Path, path_build: &Path) -> Result<()> {
        // dump cmake options
        let mut cmd = Command::new("cmake");
        cmd.arg("-LAH")
            .args(baseline_cmake_options(path_src)?)
            .arg(path_src)
            .current_dir(path_build);
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
            .args(baseline_cmake_options(path_src)?)
            .arg("-DCMAKE_EXPORT_COMPILE_COMMANDS=ON")
            .arg(path_src)
            .current_dir(path_build);
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
        fs::create_dir_all(
            artifact
                .parent()
                .ok_or_else(|| anyhow!("invalid artifact path"))?,
        )?;
        let options = dir::CopyOptions::new();
        dir::copy(path_build, artifact, &options)?;

        // done
        Ok(())
    }
}
