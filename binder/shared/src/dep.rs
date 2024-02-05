use std::fs;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Result};
use log::{info, warn};
use tempfile::tempdir;

use crate::config::{PATH_ROOT, PATH_STUDIO};
use crate::git::GitRepo;

/// A mark for dep state
static READY_MARK: &str = "ready";

/// A trait that marks an artifact resolver
pub trait Resolver: Sized {
    /// Construct a resolver from the baseline path
    fn construct(path: PathBuf) -> Self;

    /// Destruct the resolver and get back the baseline path
    fn destruct(self) -> PathBuf;

    /// Try to create a resolver from the baseline path
    fn seek() -> Result<(GitRepo, Self)>;
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

        let mark = artifact.with_extension(READY_MARK);

        // build the artifact
        fs::create_dir_all(&artifact)?;
        let resolver = R::construct(artifact);
        T::build(repo.path(), &resolver)?;

        // create the mark
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(mark)?;

        // return the package
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

        // remove the mark
        let mark = path_artifact.with_extension(READY_MARK);
        if !mark.exists() {
            bail!(
                "package artifact exists without mark: {}/{}",
                T::repo_path_from_root().join("/"),
                repo.commit()
            );
        }
        fs::remove_file(mark)?;

        // remove the artifact directory
        fs::remove_dir_all(&path_artifact)?;

        // return the scratch
        Ok(Scratch {
            repo,
            artifact: path_artifact,
            _phantom_r: PhantomData,
            _phantom_t: PhantomData,
        })
    }
}

/// Automatically differentiate the scratch and package version of LLVM
pub enum DepState<R: Resolver, T: Dependency<R>> {
    Scratch(Scratch<R, T>),
    Package(Package<R, T>),
}

impl<R: Resolver, T: Dependency<R>> DepState<R, T> {
    /// Get the deps state
    pub fn new() -> Result<Self> {
        // derive the correct path
        let segments = T::repo_path_from_root();

        let mut repo_path = PATH_ROOT.clone();
        repo_path.extend(segments);
        let repo = GitRepo::new(repo_path, None)?;
        let commit = repo.commit();

        let mut artifact = PATH_STUDIO.to_path_buf();
        artifact.extend(segments);
        artifact.push(commit);

        // a filesystem mark showing that the artifact is ready
        let ready = artifact.with_extension(READY_MARK);

        // derive the state
        let state = if ready.exists() {
            if !artifact.exists() {
                bail!(
                    "package mark exists without artifact: {}/{}",
                    segments.join("/"),
                    commit
                );
            }
            Self::Package(Package {
                repo,
                artifact: R::construct(artifact),
                _phantom: PhantomData,
            })
        } else {
            if artifact.exists() {
                info!("Deleting unsuccessful build");
                fs::remove_dir_all(&artifact)?;
            }
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

    /// Retrieve the source and artifact
    pub fn into_source_and_artifact(self) -> Result<(GitRepo, R)> {
        match self {
            Self::Scratch(_) => Err(anyhow!("package not ready")),
            Self::Package(Package { repo, artifact, .. }) => Ok((repo, artifact)),
        }
    }
}
