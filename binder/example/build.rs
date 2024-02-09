use std::env;
use std::path::PathBuf;

fn main() {
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
