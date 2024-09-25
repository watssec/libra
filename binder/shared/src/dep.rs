use std::fs;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};

use anyhow::{bail, Result};
use log::{info, warn};
use tempfile::tempdir;

use crate::config::PATH_STUDIO;

/// A mark for dep state
static READY_MARK: &str = "ready";

/// A trait that marks a dependency in the project
pub trait Dependency {
    /// Name of this dependency
    fn name() -> &'static str;

    /// Print information (e.g., configurable options) on how to build it
    fn tweak(path_wks: &Path) -> Result<()>;

    /// Build this dependency from scratch
    fn build(path_wks: &Path) -> Result<()>;
}

/// A struct that represents the build-from-scratch state
pub struct Scratch<T: Dependency> {
    path_wks: PathBuf,
    _phantom: PhantomData<T>,
}

impl<T: Dependency> Scratch<T> {
    /// Build a dependency from scratch
    pub fn make(self) -> Result<Package<T>> {
        let Self { path_wks, _phantom } = self;

        let mark = path_wks.with_extension(READY_MARK);

        // build the artifact
        fs::create_dir_all(&path_wks)?;
        T::build(&path_wks)?;

        // create the mark
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(mark)?;

        // return the package
        Ok(Package {
            path_wks,
            _phantom: PhantomData,
        })
    }
}

/// A struct that represents the package-ready state
pub struct Package<T: Dependency> {
    path_wks: PathBuf,
    _phantom: PhantomData<T>,
}

impl<T: Dependency> Package<T> {
    /// Destroy the deps so that we can build it again
    pub fn destroy(self) -> Result<Scratch<T>> {
        let Self { path_wks, _phantom } = self;

        // remove the mark
        let mark = path_wks.with_extension(READY_MARK);
        if !mark.exists() {
            bail!("package artifact exists without mark: {}", T::name());
        }
        fs::remove_file(mark)?;

        // remove the artifact directory
        fs::remove_dir_all(&path_wks)?;

        // return the scratch
        Ok(Scratch {
            path_wks,
            _phantom: PhantomData,
        })
    }
}

/// Automatically differentiate the scratch and package version of LLVM
pub enum DepState<T: Dependency> {
    Scratch(Scratch<T>),
    Package(Package<T>),
}

impl<T: Dependency> DepState<T> {
    /// Get the deps state
    pub fn new() -> Result<Self> {
        // derive the correct path
        let path_wks = PATH_STUDIO.join(T::name());

        // a filesystem mark showing that the artifact is ready
        let mark = path_wks.with_extension(READY_MARK);

        // derive the state
        let state = if mark.exists() {
            if !path_wks.exists() {
                bail!("package mark exists without artifact: {}", T::name());
            }
            Self::Package(Package {
                path_wks,
                _phantom: PhantomData,
            })
        } else {
            if path_wks.exists() {
                info!("Deleting previous build");
                fs::remove_dir_all(&path_wks)?;
            }
            Self::Scratch(Scratch {
                path_wks,
                _phantom: PhantomData,
            })
        };

        // done
        Ok(state)
    }

    /// Print information (e.g., configurable options) on how to build it
    pub fn tweak(self) -> Result<()> {
        // always happens in tmpfs
        let tmp = tempdir()?;
        T::tweak(tmp.path())?;
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
}
