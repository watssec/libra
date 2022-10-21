use log::SetLoggerError;
use simplelog::{ColorChoice, Config, LevelFilter, TermLogger, TerminalMode};

pub fn setup(verbose: bool) -> Result<(), SetLoggerError> {
    TermLogger::init(
        if verbose {
            LevelFilter::Debug
        } else {
            LevelFilter::Info
        },
        Config::default(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )
}
