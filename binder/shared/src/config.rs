use std::env;
use std::fmt::{Display, Formatter};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use lazy_static::lazy_static;
use simplelog::{ColorChoice, ConfigBuilder, LevelFilter, TermLogger, TerminalMode};

/// Name of project
pub static PROJECT: &str = "LIBRA";

// environmental configs
lazy_static! {
    // docker
    pub static ref DOCERIZED: bool = matches!(env::var("DOCKER"), Ok(val) if val == "1");

    // paths
    pub static ref PATH_ROOT: PathBuf = {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        assert!(path.pop());
        assert!(path.pop());
        path
    };
    pub static ref PATH_STUDIO: PathBuf = {
        match env::var("LIBRA_STUDIO") {
            Ok(val) if !val.is_empty() => PathBuf::from(val),
            _ => PATH_ROOT
                .join("studio")
                .join(if *DOCERIZED { "docker" } else { "native" }),
        }
    };
}

/// Marks whether initialization is completed
static INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Mode of operation
pub enum Mode {
    /// production mode
    Prod,
    /// development mode
    Dev,
    /// debug mode
    Debug,
    /// verbose mode
    Verbose,
}

impl Display for Mode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Prod => write!(f, "production"),
            Self::Dev => write!(f, "development"),
            Self::Debug => write!(f, "debug"),
            Self::Verbose => write!(f, "verbose"),
        }
    }
}

lazy_static! {
    /// Which mode to run on (default to development mode)
    pub static ref MODE: Mode = {
        let setting = env::var(format!("{}_VERBOSE", PROJECT))
            .or(env::var("VERBOSE"))
            .or(env::var("V"));
        let verbosity = match setting {
            Ok(val) => val.parse::<usize>().ok(),
            Err(_) => None,
        }.unwrap_or(1);

        match verbosity {
            0 => Mode::Prod,
            1 => Mode::Dev,
            2 => Mode::Debug,
            _ => Mode::Verbose,
        }
    };
}

/// Workspace
pub struct Workspace {
    /// path to project base
    pub base: PathBuf,
    /// path to studio directory
    pub studio: PathBuf,
}

lazy_static! {
    /// Directory layout
    pub static ref WKS: Workspace = {
        let dockerized = matches!(env::var("DOCKER"), Ok(val) if val == "1");

        // grab root path
        let mut base = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        assert!(base.pop());

        // derive other paths
        let studio = base
                .join("studio")
                .join(if dockerized { "docker" } else { "native" });

        // done
        Workspace {
            base,
            studio,
        }
    };
}

/// initialize all configs
pub fn initialize() {
    // check whether we need to run the initialization process
    match INITIALIZED.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst) {
        Ok(false) => (),
        Err(true) => return,
        _ => panic!("invalid result from atomic reading"),
    }

    // logging
    let level = match *MODE {
        Mode::Prod => LevelFilter::Warn,
        Mode::Dev => LevelFilter::Info,
        Mode::Debug => LevelFilter::Debug,
        Mode::Verbose => LevelFilter::Trace,
    };
    let mut config = ConfigBuilder::new();
    config
        .set_location_level(LevelFilter::Off)
        .set_target_level(LevelFilter::Off)
        .set_thread_level(LevelFilter::Off)
        .set_time_level(LevelFilter::Off);
    TermLogger::init(
        level,
        config.build(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )
    .expect("logging facility should be initialized");
}
