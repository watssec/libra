use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use walkdir::WalkDir;

use crate::proxy::{ClangArg, COMMAND_EXTENSION};

enum Action {
    Compile {
        input: PathBuf,
        output: PathBuf,
    },
    Link {
        inputs: Vec<PathBuf>,
        output: PathBuf,
    },
    CompileAndLink {
        input: PathBuf,
        output: PathBuf,
    },
    Assemble {
        input: PathBuf,
        output: PathBuf,
    },
}

impl Action {
    fn parse(path_src: &Path, args: &[ClangArg]) -> Result<Self> {
        // check the output
        let mut output = None;
        for item in args {
            if let ClangArg::Output(out) = item {
                if output.is_some() {
                    panic!("more than one output specified");
                }
                let out_path = Path::new(out);
                if out_path.is_absolute() {}
                output = Some(out);
            }
        }
        let path = match output {
            None => return,
            Some(out) => format!("{}{}", out, COMMAND_EXTENSION),
        };
    }
}

/// Scan over the directory and collect build commands
pub fn analyze(path_src: &Path) -> Result<()> {
    // collect commands
    for entry in WalkDir::new(path_src) {
        let entry = entry?;
        let path = entry.path();
        if path.extension().map_or(false, |e| e == COMMAND_EXTENSION) {
            let content = fs::read_to_string(path)?;
            let args: Vec<ClangArg> = serde_json::from_str(&content)?;
        }
    }

    Ok(())
}
