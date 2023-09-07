use anyhow::Result;

use libra_shared::dep::Resolver;
use libra_shared::git::GitRepo;

/// A trait that marks a test suite
pub trait TestSuite<R: Resolver> {
    /// Run the test suite
    fn run(repo: &GitRepo, resolver: R) -> Result<()>;
}
