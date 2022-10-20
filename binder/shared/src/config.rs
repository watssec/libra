use std::env;
use std::path::PathBuf;

use lazy_static::lazy_static;

// common configurations
lazy_static! {
    pub static ref DOCERIZED: bool = matches!(env::var("DOCKER"), Ok(val) if val == "1");
    pub static ref PATH_ROOT: PathBuf = {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        assert!(path.pop());
        assert!(path.pop());
        path
    };
    pub static ref PATH_STUDIO: PathBuf = {
        let mut path = PATH_ROOT.join("studio");
        if *DOCERIZED {
            path.push("docker");
        } else {
            path.push("native");
        }
        path
    };
}
