use std::path::Path;
use std::process::Command;

use anyhow::{anyhow, Result};

use libra_shared::dep::Dependency;

// path constants
static PATH_REPO: [&str; 2] = ["deps", "llvm-project"];

/// Get baseline cmake command
fn baseline_cmake_options() -> Vec<String> {
    vec![
        "-DCMAKE_BUILD_TYPE=Debug".into(),
        "-DBUILD_SHARED_LIBS=ON".into(),
        format!(
            "-DLLVM_ENABLE_PROJECTS={}",
            [
                "clang",
                "clang-tools-extra",
                "libc",
                "compiler-rt",
                "lld",
                "lldb",
                "polly",
                "mlir",
            ]
            .join(";")
        ),
        format!(
            "-DLLVM_ENABLE_RUNTIMES={}",
            ["libcxx", "libcxxabi"].join(";")
        ),
        "-DLLVM_ENABLE_RTTI=ON".into(),
        "-DLIBC_ENABLE_USE_BY_CLANG=ON".into(),
        "-DCLANG_DEFAULT_CXX_STDLIB=libc++".into(),
        #[cfg(target_os = "macos")]
        "-DCMAKE_OSX_ARCHITECTURES=arm64".into(),
    ]
}

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
            .args(baseline_cmake_options())
            .arg(path_src.join("llvm"))
            .current_dir(path_build);
        let status = cmd.status()?;
        if !status.success() {
            return Err(anyhow!("Configure failed"));
        }

        // done
        Ok(())
    }

    fn build(path_src: &Path, path_build: &Path, path_install: Option<&Path>) -> Result<()> {
        let artifact = path_install.ok_or_else(|| anyhow!("No artifact path"))?;

        // llvm configuration
        let mut cmd = Command::new("cmake");
        cmd.arg("-G")
            .arg("Ninja")
            .args(baseline_cmake_options())
            .arg(path_src.join("llvm"))
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
