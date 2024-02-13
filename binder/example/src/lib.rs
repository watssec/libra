use anyhow::Result;
use structopt::StructOpt;

use libra_shared::config::initialize;

use crate::workflow::execute;

pub mod proxy;

mod apps;
mod common;
mod snippet;
mod wllvm;
mod workflow;

#[derive(StructOpt)]
enum Example {
    ApacheHttpd,
    LibXML2,
    PCRE2,
    ZLib,
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
        Example::LibXML2 => execute::<apps::libxml2::Config>(),
        Example::PCRE2 => execute::<apps::pcre2::Config>(),
        Example::ZLib => execute::<apps::zlib::Config>(),
    }
}
