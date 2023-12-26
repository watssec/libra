use anyhow::Result;
use structopt::StructOpt;

use libra_shared::logging;

#[derive(StructOpt)]
enum Example {
    ApacheHttpd,
}

#[derive(StructOpt)]
#[structopt(
    name = "libra-example",
    about = "A driver for LIBRA workflow on example projects",
    rename_all = "kebab-case"
)]
struct Args {
    /// Verbosity
    #[structopt(short, long)]
    verbose: Option<usize>,

    /// Example
    #[structopt(subcommand)]
    example: Example,
}

/// Main entrypoint
pub fn entrypoint() -> Result<()> {
    let args = Args::from_args();
    let Args { verbose, example } = args;
    // setup logging
    logging::setup(verbose)?;

    // run the subcommand
    match example {
        Example::ApacheHttpd => (),
    }

    Ok(())
}
