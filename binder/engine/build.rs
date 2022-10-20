use std::env;
use std::path::PathBuf;

use libra_builder::deps::artifact_for_llvm;
use libra_shared::config::PATH_STUDIO;

fn main() {
    // paths
    let dir_studio = env::var("LIBRA_STUDIO")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PATH_STUDIO.to_path_buf());

    // llvm: package
    let opt_llvm_version = env::var("LIBRA_CONFIG_LLVM_VERSION").ok();
    let pkg_llvm = artifact_for_llvm(&dir_studio, opt_llvm_version.as_deref()).unwrap();
    println!(
        "cargo:rustc-env=LIBRA_CONST_LLVM_ARTIFACT={}",
        pkg_llvm.to_str().unwrap()
    );
}
