use std::path::Path;

use anyhow::Result;

use libra_shared::git::GitRepo;

/// A trait that marks a test suite
pub trait TestSuite {
    /// Run the test suite
    fn run(repo: &GitRepo, path_artifact: &Path) -> Result<()>;
}
