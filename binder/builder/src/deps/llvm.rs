use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{anyhow, Result};
use libra_shared::config::PROJECT;

use libra_shared::dep::{DepState, Dependency, Resolver};
use libra_shared::git::GitRepo;

// path constants
static PATH_REPO: [&str; 2] = ["deps", "llvm-project"];

// default cmake cache to use
static CMAKE_CACHE: &str = include_str!("llvm.cmake");

/// Artifact path resolver for LLVM
pub struct ResolverLLVM {
    /// Base path for the artifact directory
    path_artifact: PathBuf,
    /// <artifact>/build
    path_build: PathBuf,
    /// <artifact>/build
    path_install: PathBuf,
}

impl Resolver for ResolverLLVM {
    fn construct(path: PathBuf) -> Self {
        Self {
            path_build: path.join("build"),
            path_install: path.join("install"),
            path_artifact: path,
        }
    }

    fn destruct(self) -> PathBuf {
        self.path_artifact
    }

    fn seek() -> Result<(GitRepo, Self)> {
        DepState::<ResolverLLVM, DepLLVM>::new()?.into_source_and_artifact()
    }
}

impl ResolverLLVM {
    pub fn path_build(&self) -> &Path {
        &self.path_build
    }

    pub fn path_install(&self) -> &Path {
        &self.path_install
    }
}

/// Represent the LLVM deps
pub struct DepLLVM {}

impl Dependency<ResolverLLVM> for DepLLVM {
    fn repo_path_from_root() -> &'static [&'static str] {
        &PATH_REPO
    }

    fn list_build_options(path_src: &Path, path_config: &Path) -> Result<()> {
        // dump the cmake cache
        let path_cmake_cache = path_src.join(format!("{}.cmake", PROJECT));
        fs::write(&path_cmake_cache, CMAKE_CACHE)?;

        // cmake list options against the cache
        let mut cmd = Command::new("cmake");
        cmd.arg("-LAH")
            .arg("-C")
            .arg(&path_cmake_cache)
            .arg(path_src.join("llvm"))
            .current_dir(path_config);
        let status = cmd.status()?;
        if !status.success() {
            return Err(anyhow!("Configure failed"));
        }

        // clean up the cmake cache
        fs::remove_file(path_cmake_cache)?;
        Ok(())
    }

    fn build(path_src: &Path, resolver: &ResolverLLVM) -> Result<()> {
        // dump the cmake cache
        let path_cmake_cache = path_src.join(format!("{}.cmake", PROJECT));
        fs::write(&path_cmake_cache, CMAKE_CACHE)?;

        // config
        fs::create_dir(&resolver.path_build)?;
        let mut cmd = Command::new("cmake");
        cmd.arg("-G")
            .arg("Ninja")
            .arg("-C")
            .arg(&path_cmake_cache)
            .arg(path_src.join("llvm"))
            .current_dir(&resolver.path_build);
        let status = cmd.status()?;
        if !status.success() {
            return Err(anyhow!("Configure failed"));
        }

        // build
        let mut cmd = Command::new("cmake");
        cmd.arg("--build")
            .arg(&resolver.path_build)
            .arg("--parallel")
            .arg("1")
            .arg("--target")
            .arg("stage3");
        let status = cmd.status()?;
        if !status.success() {
            return Err(anyhow!("Build failed"));
        }

        // install
        fs::create_dir(&resolver.path_install)?;

        let mut cmd = Command::new("cmake");
        cmd.arg("--install")
            .arg(&resolver.path_build)
            .arg("--prefix")
            .arg(&resolver.path_install)
            .arg("--target")
            .arg("stage3-install");
        let status = cmd.status()?;
        if !status.success() {
            return Err(anyhow!("Install failed"));
        }

        // clean up the cmake cache
        fs::remove_file(path_cmake_cache)?;
        Ok(())
    }
}
