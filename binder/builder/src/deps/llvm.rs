use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{anyhow, bail, Result};

use libra_shared::config::{PATH_ROOT, PROJECT};
use libra_shared::dep::{DepState, Dependency};
use libra_shared::git::GitRepo;

// default cmake cache to use
static CMAKE_CACHE: &str = include_str!("llvm.cmake");

/// Information to be consumed while building it
struct PrepResult {
    path_src_llvm: PathBuf,
    path_src_cmake_cache: PathBuf,
    path_build: PathBuf,
    path_install: PathBuf,
}

/// Represent the LLVM dependency
pub struct DepLLVM {}

impl DepLLVM {
    /// Prepare the stage for build
    fn prep(path_wks: &Path) -> Result<PrepResult> {
        let path_src = path_wks.join("src");

        // checkout
        let mut repo = GitRepo::new(PATH_ROOT.join("deps").join("llvm-project"), None)?;
        repo.checkout(&path_src)?;

        // dump the cmake cache
        let path_src_cmake_cache = path_src.join(format!("{}.cmake", PROJECT));
        fs::write(&path_src_cmake_cache, CMAKE_CACHE)?;

        // prepare for the build and install directory
        let path_build = path_wks.join("build");
        fs::create_dir(&path_build)?;

        let path_install = path_wks.join("install");
        fs::create_dir(&path_install)?;

        // done
        Ok(PrepResult {
            path_src_llvm: path_src.join("llvm"),
            path_src_cmake_cache,
            path_build,
            path_install,
        })
    }
}

impl Dependency for DepLLVM {
    fn name() -> &'static str {
        "llvm"
    }

    fn tweak(path_wks: &Path) -> Result<()> {
        // prepare the source code
        let pack = Self::prep(path_wks)?;

        // cmake list options against the cache
        let mut cmd = Command::new("cmake");
        cmd.arg("-LAH")
            .arg("-C")
            .arg(&pack.path_src_cmake_cache)
            .arg(&pack.path_src_llvm)
            .current_dir(&pack.path_build);
        let status = cmd.status()?;
        if !status.success() {
            bail!("Configure failed with status {}", status);
        }

        // done
        Ok(())
    }

    fn build(path_wks: &Path) -> Result<()> {
        // prepare the source code
        let pack = Self::prep(path_wks)?;

        // config
        let mut cmd = Command::new("cmake");
        cmd.arg("-G")
            .arg("Ninja")
            .arg("-C")
            .arg(&pack.path_src_cmake_cache)
            .arg(format!(
                "-DCMAKE_INSTALL_PREFIX={}",
                pack.path_install
                    .to_str()
                    .ok_or_else(|| anyhow!("non-ascii path"))?
            ))
            .arg(&pack.path_src_llvm)
            .current_dir(&pack.path_build);
        let status = cmd.status()?;
        if !status.success() {
            bail!("Configure failed with status {}", status);
        }

        // build
        let mut cmd = Command::new("cmake");
        cmd.arg("--build")
            .arg(&pack.path_build)
            .arg("--target")
            .arg("stage3");
        let status = cmd.status()?;
        if !status.success() {
            bail!("Build failed with status {}", status);
        }

        // install
        let mut cmd = Command::new("cmake");
        cmd.arg("--build")
            .arg(&pack.path_build)
            .arg("--target")
            .arg("stage3-install");
        let status = cmd.status()?;
        if !status.success() {
            bail!("Install failed with status {}", status);
        }

        // done
        Ok(())
    }
}

/// Artifact to be used in LLVM pass building
#[non_exhaustive]
pub struct ArtifactLLVM {
    pub path_src: PathBuf,
    pub path_build: PathBuf,
    pub path_install: PathBuf,
}

impl ArtifactLLVM {
    pub fn seek() -> Result<Self> {
        let path_wks = DepState::<DepLLVM>::new()?.artifact()?;
        Ok(Self {
            path_src: path_wks.join("src"),
            path_build: path_wks.join("build"),
            path_install: path_wks.join("install"),
        })
    }
}
