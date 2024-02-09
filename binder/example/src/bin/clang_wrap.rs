use std::process::Command;
use std::{env, process};

use libra_engine::flow::shared::Context;

fn main() {
    let ctxt = Context::new().expect("LLVM context");
    let bin_clang = ctxt.path_llvm(["bin", "clang"]).expect("ascii path only");
    let status = Command::new(bin_clang)
        .args(env::args().skip(1))
        .status()
        .expect("command execution");
    process::exit(status.code().expect("status code"))
}
