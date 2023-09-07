use std::fs;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};
use log::{info, warn};
use tempfile::tempdir;

use crate::config::PATH_ROOT;
use crate::git::GitRepo;

/// A trait that marks an artifact resolver
pub trait Resolver: Sized {
    /// Construct a resolver from the baseline path
    fn construct(path: PathBuf) -> Self;

    /// Destruct the resolver and get back the baseline path
    fn destruct(self) -> PathBuf;

    /// Try to create a resolver from the baseline path
    fn seek(studio: &Path, version: Option<&str>) -> Result<Self>;
}

/// A trait that marks a dependency in the project
pub trait Dependency<R: Resolver> {
    /// Location of the git repo from the project root
    fn repo_path_from_root() -> &'static [&'static str];

    /// List configurable options for building
    fn list_build_options(path_src: &Path, path_config: &Path) -> Result<()>;

    /// Build the deps from scratch
    fn build(path_src: &Path, resolver: &R) -> Result<()>;
}

/// A struct that represents the build-from-scratch state
pub struct Scratch<R: Resolver, T: Dependency<R>> {
    repo: GitRepo,
    artifact: PathBuf,
    _phantom_r: PhantomData<R>,
    _phantom_t: PhantomData<T>,
}

impl<R: Resolver, T: Dependency<R>> Scratch<R, T> {
    /// Build the deps from scratch
    pub fn make(self) -> Result<Package<R, T>> {
        let Self {
            repo,
            artifact,
            _phantom_r,
            _phantom_t,
        } = self;

        fs::create_dir_all(&artifact)?;
        let resolver = R::construct(artifact);
        T::build(repo.path(), &resolver)?;

        Ok(Package {
            repo,
            artifact: resolver,
            _phantom: PhantomData,
        })
    }
}

/// A struct that represents the package-ready state
pub struct Package<R: Resolver, T: Dependency<R>> {
    repo: GitRepo,
    artifact: R,
    _phantom: PhantomData<T>,
}

impl<R: Resolver, T: Dependency<R>> Package<R, T> {
    /// Destroy the deps so that we can build it again
    pub fn destroy(self) -> Result<Scratch<R, T>> {
        let Self {
            repo,
            artifact,
            _phantom,
        } = self;

        let path_artifact = artifact.destruct();
        fs::remove_dir_all(&path_artifact)?;

        Ok(Scratch {
            repo,
            artifact: path_artifact,
            _phantom_r: PhantomData,
            _phantom_t: PhantomData,
        })
    }

    /// Get the git repo from the package
    pub fn git_repo(&self) -> &GitRepo {
        &self.repo
    }
}

/// Automatically differentiate the scratch and package version of LLVM
pub enum DepState<R: Resolver, T: Dependency<R>> {
    Scratch(Scratch<R, T>),
    Package(Package<R, T>),
}

impl<R: Resolver, T: Dependency<R>> DepState<R, T> {
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
                artifact: R::construct(artifact),
                _phantom: PhantomData,
            })
        } else {
            Self::Scratch(Scratch {
                repo,
                artifact,
                _phantom_r: PhantomData,
                _phantom_t: PhantomData,
            })
        };

        // done
        Ok(state)
    }

    /// List the possible build options
    pub fn list_build_options(self) -> Result<()> {
        let repo = match self {
            Self::Scratch(Scratch { repo, .. }) => repo,
            Self::Package(Package { repo, .. }) => repo,
        };

        // always happens in tmpfs
        let tmp = tempdir()?;
        T::list_build_options(repo.path(), tmp.path())?;
        tmp.close()?;

        Ok(())
    }

    /// Build the package
    pub fn build(self, force: bool) -> Result<()> {
        let scratch = match self {
            DepState::Scratch(scratch) => scratch,
            DepState::Package(package) => {
                if !force {
                    info!("Package already exists");
                    return Ok(());
                } else {
                    warn!("Force rebuilding package");
                    package.destroy()?
                }
            }
        };
        scratch.make()?;
        Ok(())
    }

    /// Retrieve the artifact resolver
    pub fn into_artifact_resolver(self) -> Result<R> {
        match self {
            Self::Scratch(_) => Err(anyhow!("package not ready")),
            Self::Package(pkg) => Ok(pkg.artifact),
        }
    }
}
