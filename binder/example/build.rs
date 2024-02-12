use std::env;
use std::path::PathBuf;
use std::process::Command;

static WRARPS: [&str; 2] = ["clang_wrap", "clang_cpp_wrap"];

fn build_wrap(name: &str) {
    let status = Command::new(env!("CARGO"))
        .args(["build", "--bin", name])
        .status()
        .unwrap_or_else(|e| panic!("failed to spawn for {}: {}", name, e));
    if !status.success() {
        panic!("failed to build {}", name);
    }
}

fn main() {
    // compile wrappers first
    let target = env!("CARGO_CRATE_NAME");
    if target != "build_script_build" && !WRARPS.contains(&target) {
        for name in WRARPS {
            build_wrap(name);
        }
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
