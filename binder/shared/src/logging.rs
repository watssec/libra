use std::sync::atomic::{AtomicUsize, Ordering};

use log::{trace, SetLoggerError};
use simplelog::{ColorChoice, Config, LevelFilter, TermLogger, TerminalMode};

use crate::config::PARALLEL;

/// Records the current depth of the tracer
static TRACE_DEPTH: AtomicUsize = AtomicUsize::new(0);

/// Tracer representing the context
pub struct Tracer {
    title: String,
    depth: Option<usize>,
}

impl Tracer {
    /// Create a tracing session
    pub fn new(title: String) -> Self {
        let depth = if *PARALLEL {
            None
        } else {
            let level = TRACE_DEPTH.fetch_add(1, Ordering::SeqCst);
            trace!("{}-> {}", "  ".repeat(level), title);
            Some(level)
        };
        Self { title, depth }
    }

    /// Record a new event
    pub fn log(&self, event: &str) {
        match &self.depth {
            None => (),
            Some(level) => trace!("{} {}", "  ".repeat(*level), event),
        }
    }
}

impl Drop for Tracer {
    fn drop(&mut self) {
        let Self { title, depth } = self;
        match depth {
            None => (),
            Some(level) => {
                trace!("{}<- {}", "  ".repeat(*level), title);
                TRACE_DEPTH
                    .compare_exchange(*level + 1, *level, Ordering::SeqCst, Ordering::SeqCst)
                    .expect("global TRACE_DEPTH is out of sync");
            }
        }
    }
}

/// Setup the logging globally
pub fn setup(verbose: Option<usize>) -> Result<(), SetLoggerError> {
    let verbosity = match verbose.unwrap_or(0) {
        0 => LevelFilter::Info,
        1 => LevelFilter::Debug,
        _ => LevelFilter::Trace,
    };
    TermLogger::init(
        verbosity,
        Config::default(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )
}
