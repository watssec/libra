use std::env;
use std::path::PathBuf;
use std::process::Command;

use lazy_static::lazy_static;

// common configurations
lazy_static! {
    //
    // paths
    //

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

    //
    // platform configurations
    //

    pub static ref UNAME_HARDWARE: String = {
        let cmd = Command::new("uname").arg("-m").output().expect("uname");
        if !cmd.status.success() || !cmd.stderr.is_empty() {
            panic!("uname");
        }
        String::from_utf8(cmd.stdout).expect("uname").trim().to_string()
    };
    pub static ref UNAME_PLATFORM: String = {
        let cmd = Command::new("uname").arg("-s").output().expect("uname");
        if !cmd.status.success() || !cmd.stderr.is_empty() {
            panic!("uname");
        }
        String::from_utf8(cmd.stdout).expect("uname").trim().to_string()
    };
}

pub const TMPDIR_IN_STUDIO: &str = "tmp";
