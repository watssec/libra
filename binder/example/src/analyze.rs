use std::fs;
use std::path::Path;

use anyhow::Result;
use walkdir::WalkDir;

use crate::proxy::{ClangArg, COMMAND_EXTENSION};

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
