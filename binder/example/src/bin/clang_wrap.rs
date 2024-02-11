use std::process::Command;
use std::{env, fs, process};

use libra_engine::flow::shared::Context;
use libra_example::proxy::{ClangArg, ClangInvocation, COMMAND_EXTENSION};

fn main() {
    // get paths
    let ctxt = Context::new().expect("LLVM context");
    let bin_clang = ctxt.path_llvm(["bin", "clang"]).expect("ascii path only");

    // collect arguments
    let args: Vec<_> = env::args().skip(1).collect();

    // pass-through the arguments and execute the command first
    let status = Command::new(bin_clang)
        .args(&args)
        .status()
        .expect("command execution");
    if !status.success() {
        process::exit(status.code().expect("status code"))
    }

    // only process arguments upon successful invocation
    let parsed = ClangArg::collect(args.iter().map(|s| s.as_str()));

    // check output
    let mut output = None;
    for item in &parsed {
        if let ClangArg::Output(out) = item {
            if output.is_some() {
                panic!("more than one output specified: {}", args.join(" "));
            }
            output = Some(out);
        }
    }
    let path = match output {
        None => return,
        Some(out) => format!("{}{}", out, COMMAND_EXTENSION),
    };

    // create the invocation package
    let invocation = ClangInvocation {
        cwd: env::current_dir()
            .expect("unable to get current working directory")
            .canonicalize()
            .expect("unable to get canonicalize cwd path"),
        args: parsed,
    };

    // serialize
    let content = serde_json::to_string_pretty(&invocation).expect("serialization error");
    fs::write(path, content).expect("IO error");
}
