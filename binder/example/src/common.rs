use std::path::{Path, PathBuf};

use anyhow::Result;
use lazy_static::lazy_static;
use serde::de::DeserializeOwned;
use serde::Serialize;

lazy_static! {
    pub static ref CLANG_WRAP: String = {
        let mut target_dir = PathBuf::from(env!("LIBRA_TARGET_DIR"));
        target_dir.push("clang_wrap");
        target_dir
            .into_os_string()
            .into_string()
            .expect("ASCII path only")
    };
    pub static ref CLANG_CPP_WRAP: String = {
        let mut target_dir = PathBuf::from(env!("LIBRA_TARGET_DIR"));
        target_dir.push("clang_cpp_wrap");
        target_dir
            .into_os_string()
            .into_string()
            .expect("ASCII path only")
    };
}

/// Extension for LLVM IR (bitcode) file
pub static BITCODE_EXTENSION: &str = "bc";

/// Common trait for workflow config
pub trait AppConfig: Serialize + DeserializeOwned {
    /// Obtain the application name
    fn app() -> &'static str;

    /// Build process
    fn build(&self, path_src: &Path, path_bin: &Path) -> Result<()>;
}

/// Utility: Derive the filename for bitcode based on an existing file
pub fn derive_bitcode_path<P: AsRef<Path>>(path: P) -> PathBuf {
    let path = path.as_ref();
    let new_ext = path.extension().map_or_else(
        || BITCODE_EXTENSION.to_string(),
        |e| {
            format!(
                "{}.{}",
                e.to_str().expect("pure ASCII extension"),
                BITCODE_EXTENSION
            )
        },
    );
    path.with_extension(new_ext)
}
