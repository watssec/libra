use std::fs;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};

use anyhow::{bail, Result};
use log::{info, warn};
use tempfile::tempdir;

use crate::config::{PATH_ROOT, TMPDIR_IN_STUDIO};
use crate::git::GitRepo;

/// A trait that marks a dependency in the project
pub trait Dependency {
    /// Location of the git repo from the project root
    fn repo_path_from_root() -> &'static [&'static str];

    /// List configurable options for building
    fn list_build_options(path_src: &Path, path_build: &Path) -> Result<()>;

    /// Build the deps from scratch
    fn build(path_src: &Path, path_build: &Path, artifact: &Path) -> Result<()>;
}

/// A struct that represents the build-from-scratch state
pub struct Scratch<T: Dependency> {
    repo: GitRepo,
    studio: PathBuf,
    artifact: PathBuf,
    _phantom: PhantomData<T>,
}

impl<T: Dependency> Scratch<T> {
    fn path_studio(&self) -> &Path {
        self.studio.as_path()
    }

    /// Build the deps from scratch
    pub fn make(self, tmpdir: &Path) -> Result<Package<T>> {
        let Self {
            mut repo,
            studio,
            artifact,
            _phantom,
        } = self;

        // prepare source code
        let path_src = tmpdir.join("src");
        repo.checkout(&path_src)?;

        // build
        let path_build = tmpdir.join("build");
        fs::create_dir(&path_build)?;
        T::build(&path_src, &path_build, &artifact)?;

        // done with the building procedure
        Ok(Package {
            repo,
            studio,
            artifact,
            _phantom,
        })
    }
}

/// A struct that represents the package-ready state
pub struct Package<T: Dependency> {
    repo: GitRepo,
    studio: PathBuf,
    artifact: PathBuf,
    _phantom: PhantomData<T>,
}

impl<T: Dependency> Package<T> {
    fn path_studio(&self) -> &Path {
        self.studio.as_path()
    }

    /// Destroy the deps so that we can build it again
    pub fn destroy(self) -> Result<Scratch<T>> {
        let Self {
            repo,
            studio,
            artifact,
            _phantom,
        } = self;
        fs::remove_dir_all(&artifact)?;
        Ok(Scratch {
            repo,
            studio,
            artifact,
            _phantom,
        })
    }

    /// Get the git repo from the package
    pub fn git_repo(&self) -> &GitRepo {
        &self.repo
    }

    /// Get the artifact path from the package
    pub fn artifact_path(&self) -> &Path {
        &self.artifact
    }
}

/// Automatically differentiate the scratch and package version of LLVM
pub enum DepState<T: Dependency> {
    Scratch(Scratch<T>),
    Package(Package<T>),
}

impl<T: Dependency> DepState<T> {
    /// Get the deps state
    pub fn new(studio: &Path, version: Option<&str>) -> Result<Self> {
        // derive the correct path
        let segments = T::repo_path_from_root();

        let mut repo_path = PATH_ROOT.clone();
        repo_path.extend(segments);
        let repo = GitRepo::new(repo_path, version)?;

        let mut artifact = studio.to_path_buf();
        artifact.extend(segments);
        artifact.push(repo.commit());

        // check the existence of the pre-built package
        let state = if artifact.exists() {
            Self::Package(Package {
                repo,
                studio: studio.to_path_buf(),
                artifact,
                _phantom: PhantomData,
            })
        } else {
            Self::Scratch(Scratch {
                repo,
                studio: studio.to_path_buf(),
                artifact,
                _phantom: PhantomData,
            })
        };

        // done
        Ok(state)
    }

    fn path_studio(&self) -> &Path {
        match self {
            Self::Package(package) => package.path_studio(),
            Self::Scratch(scratch) => scratch.path_studio(),
        }
    }

    /// List the possible build options
    fn list_build_options(&mut self, tmpdir: &Path) -> Result<()> {
        let repo = match self {
            Self::Scratch(Scratch { repo, .. }) => repo,
            Self::Package(Package { repo, .. }) => repo,
        };

        // prepare source code
        let path_src = tmpdir.join("src");
        repo.checkout(&path_src)?;

        // list the build options
        let path_build = tmpdir.join("build");
        fs::create_dir(&path_build)?;
        T::list_build_options(&path_src, &path_build)?;

        // everything is good
        Ok(())
    }

    /// Build the package
    pub fn build(mut self, use_tmpdir: bool, config: bool, force: bool) -> Result<()> {
        // prepare the tmpdir first
        let tmpwks = if use_tmpdir {
            Ok(tempdir()?)
        } else {
            let path = self.path_studio().join(TMPDIR_IN_STUDIO);
            if path.exists() {
                if !force {
                    bail!("Tmpdir {} already exists", path.to_str().unwrap());
                }
                fs::remove_dir_all(&path)?;
            }
            fs::create_dir_all(&path)?;
            Err(path)
        };
        let tmpdir = match &tmpwks {
            Ok(dir) => dir.path(),
            Err(path) => path.as_path(),
        };

        // case on config
        if config {
            self.list_build_options(tmpdir)?;
        } else {
            match self {
                DepState::Scratch(scratch) => {
                    scratch.make(tmpdir)?;
                }
                DepState::Package(package) => {
                    if !force {
                        info!("Package already exists");
                    } else {
                        warn!("Force rebuilding package");
                        let scratch = package.destroy()?;
                        scratch.make(tmpdir)?;
                    }
                }
            }
        }

        // clean-up the temporary directory
        match tmpwks {
            Ok(_) => {}
            Err(path) => {
                fs::remove_dir_all(path)?;
            }
        }
        Ok(())
    }
}
