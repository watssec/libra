use anyhow::Result;

use libra_shared::dep::Resolver;

/// A trait that marks a test suite
pub trait TestSuite<R: Resolver> {
    /// Run the test suite
    fn run(resolver: R, force: bool) -> Result<()>;
}
