use std::env;
use std::path::PathBuf;
use std::process::Command;

fn build_wrapper(name: &str) {
    let status = Command::new(env!("CARGO"))
        .args(["build", "--bin", name])
        .status()
        .unwrap_or_else(|e| panic!("failed to spawn for {}: {}", name, e));
    if !status.success() {
        panic!("failed to build {}", name);
    }
}

fn main() {
    // decide whether we are compiling the main
    let is_main_or_default = match env::var_os("CARGO_BIN_NAME") {
        None => env::var_os("CARGO_PRIMARY_PACKAGE").is_some(),
        Some(e) => e.to_str().map_or(false, |n| n == env!("CARGO_PKG_NAME")),
    };

    // compile wrappers first
    if is_main_or_default {
        build_wrapper("clang_wrap");
        build_wrapper("clang_cpp_wrap");
    }

    // tweak the environment variables
    let mut out_dir =
        PathBuf::from(env::var_os("OUT_DIR").expect("environment variable OUT_DIR is not set"));
    for _ in 0..3 {
        if !out_dir.pop() {
            panic!("unable to retrieve parent of OUT_DIR");
        }
    }
    let target_dir = out_dir
        .into_os_string()
        .into_string()
        .expect("ASCII path only");

    println!("cargo:rustc-env=LIBRA_TARGET_DIR={}", target_dir);
}
