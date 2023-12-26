mod apps;
mod common;

use anyhow::Result;
use structopt::StructOpt;

use crate::common::execute;
use libra_shared::config::initialize;

#[derive(StructOpt)]
enum Example {
    ApacheHttpd,
    PCRE2,
}

#[derive(StructOpt)]
#[structopt(
    name = "libra-example",
    about = "A driver for LIBRA workflow on example projects",
    rename_all = "kebab-case"
)]
struct Args {
    /// Example
    #[structopt(subcommand)]
    example: Example,
}

/// Main entrypoint
pub fn entrypoint() -> Result<()> {
    // setup
    let args = Args::from_args();
    let Args { example } = args;
    initialize();

    // run the subcommand
    match example {
        Example::ApacheHttpd => execute::<apps::apache_httpd::Config>(),
        Example::PCRE2 => execute::<apps::pcre2::Config>(),
    }
}
