use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    // skip environment variable tweaking for wrappers
    match env::var_os("CARGO_BIN_NAME")
        .as_ref()
        .and_then(|e| e.to_str())
    {
        None => (),
        Some(name) => {
            match name {
                "clang_wrap" => return,
                "libra_example" => {
                    // compile wrappers first
                    let status = Command::new(env!("CARGO"))
                        .args(["build", "--bin", "clang_wrap"])
                        .status()
                        .expect("spawn to compile clang_wrap");
                    if !status.success() {
                        panic!("failed to compile clang_wrap");
                    }
                }
                _ => (),
            }
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
