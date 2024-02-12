use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::{fs, io};

use anyhow::{bail, Result};
use libra_engine::flow::shared::Context;
use log::debug;
use petgraph::algo::toposort;
use petgraph::graph::DiGraph;
use walkdir::WalkDir;

use crate::proxy::{ClangArg, ClangInvocation, COMMAND_EXTENSION, LIBMARK_EXTENSION};

static BITCODE_EXTENSION: &str = "bc";

enum SysLib {
    C,
    Math,
    POSIXThread,
}

enum CommonExtensions {
    C,
    CPP,
    Asm,
    Object,
    LibStatic,
    LibShared,
}

impl CommonExtensions {
    pub fn probe(path: &Path) -> Option<Self> {
        let lang = match path.extension().and_then(|e| e.to_str())? {
            "c" => Self::C,
            "cpp" => Self::CPP,
            "cc" => Self::CPP,
            "s" => Self::Asm,
            "o" => Self::Object,
            "a" => Self::LibStatic,
            "so" => Self::LibShared,
            _ => return None,
        };
        Some(lang)
    }
}

#[derive(Default)]
struct Libraries {
    sys: Vec<SysLib>,
    usr: Vec<PathBuf>,
}

enum Action {
    Compile {
        input: PathBuf,
        output: PathBuf,
        invocation: ClangInvocation,
    },
    Link {
        inputs: Vec<PathBuf>,
        libs: Libraries,
        output: PathBuf,
        invocation: ClangInvocation,
    },
    CompileAndLink {
        input: PathBuf,
        libs: Libraries,
        output: PathBuf,
        invocation: ClangInvocation,
    },
}

impl Action {
    fn filter_args_for_output(invocation: ClangInvocation) -> Result<(ClangInvocation, PathBuf)> {
        let ClangInvocation { cwd, cxx, args } = invocation;
        let mut new_args = vec![];

        // find the output
        let mut target = None;
        for item in args {
            if let ClangArg::Output(name) = &item {
                if target.is_some() {
                    panic!("more than one output specified");
                }

                // resolve path
                let path_resolved = normalize_path(&cwd, name);
                if !path_resolved.exists() {
                    bail!("output path does not exist");
                }
                target = Some(path_resolved);
            } else {
                new_args.push(item);
            }
        }

        // check that output exists
        let output = match target {
            None => bail!("no output in the invocation"),
            Some(out) => out,
        };

        // repack
        let new_invocation = ClangInvocation {
            cwd,
            cxx,
            args: new_args,
        };
        Ok((new_invocation, output))
    }

    fn filter_args_for_inputs(
        invocation: ClangInvocation,
    ) -> Result<(ClangInvocation, Vec<PathBuf>)> {
        let ClangInvocation { cwd, cxx, args } = invocation;
        let mut new_args = vec![];

        // find the inputs
        let mut inputs = vec![];
        for item in args {
            if let ClangArg::Input(name) = &item {
                // resolve path
                let path_resolved = normalize_path(&cwd, name);
                if !path_resolved.exists() {
                    bail!("input path does not exist");
                }
                inputs.push(path_resolved);
            } else {
                new_args.push(item);
            }
        }

        // check that inputs are not empty
        if inputs.is_empty() {
            bail!("no inputs in the invocation");
        }

        // repack
        let new_invocation = ClangInvocation {
            cwd,
            cxx,
            args: new_args,
        };
        Ok((new_invocation, inputs))
    }

    fn filter_args_for_mode_compile(
        invocation: ClangInvocation,
    ) -> Result<(ClangInvocation, bool)> {
        let ClangInvocation { cwd, cxx, args } = invocation;
        let mut new_args = vec![];

        // look for flag
        let mut is_compile_only = false;
        for item in args {
            if matches!(&item, ClangArg::ModeCompile) {
                if is_compile_only {
                    bail!("-c specified multiple times");
                }
                is_compile_only = true;
            } else {
                new_args.push(item);
            }
        }

        // repack
        let new_invocation = ClangInvocation {
            cwd,
            cxx,
            args: new_args,
        };
        Ok((new_invocation, is_compile_only))
    }

    fn filter_args_for_mode_link(
        invocation: ClangInvocation,
    ) -> Result<(ClangInvocation, Option<Libraries>)> {
        let ClangInvocation { cwd, cxx, args } = invocation;
        let mut new_args = vec![];

        // collect libraries
        let mut has_linking_flags = false;
        let mut lib_names = vec![];
        let mut lib_paths = vec![];
        let mut libs_sys = vec![];

        for item in args {
            match &item {
                ClangArg::LibName(val) => {
                    has_linking_flags = true;

                    // resolve system libraries
                    match val.as_str() {
                        "c" => libs_sys.push(SysLib::C),
                        "m" => libs_sys.push(SysLib::Math),
                        "pthread" => libs_sys.push(SysLib::POSIXThread),
                        _ => lib_names.push(val.to_string()),
                    }
                }
                ClangArg::LibPath(val) => {
                    has_linking_flags = true;

                    // resolve path
                    let path = Path::new(val);
                    let path_resolved = if path.is_absolute() {
                        path.to_path_buf()
                    } else {
                        cwd.join(path)
                    };
                    if path_resolved.exists() {
                        lib_paths.push(path_resolved);
                    }
                }
                ClangArg::LinkStatic
                | ClangArg::LinkShared
                | ClangArg::LinkRpath(..)
                | ClangArg::LinkSoname(..) => {
                    has_linking_flags = true;
                }
                _ => {
                    new_args.push(item);
                }
            }
        }

        // find requested libraries
        let libs = if has_linking_flags {
            let mut libs_usr = vec![];
            for name in lib_names {
                let mark = format!("lib{}{}", name, LIBMARK_EXTENSION);

                let mut found = false;
                for path in &lib_paths {
                    for entry in fs::read_dir(path)? {
                        let entry = entry?;
                        if entry.file_name().into_string().map_or(false, |e| e == mark) {
                            if found {
                                bail!("more than one candidate found for library {}", name);
                            }
                            found = true;
                            // TODO: deref the mark
                            libs_usr.push(entry.path());
                        }
                    }
                }
                if !found {
                    bail!("library {} not found", name);
                }
            }
            Some(Libraries {
                sys: libs_sys,
                usr: libs_usr,
            })
        } else {
            None
        };

        // repack
        let new_invocation = ClangInvocation {
            cwd,
            cxx,
            args: new_args,
        };
        Ok((new_invocation, libs))
    }

    fn parse(invocation: ClangInvocation) -> Result<Self> {
        let (invocation, output) = Self::filter_args_for_output(invocation)?;
        let (invocation, inputs) = Self::filter_args_for_inputs(invocation)?;
        let (invocation, is_compile_only) = Self::filter_args_for_mode_compile(invocation)?;
        let (invocation, link_libs_opt) = Self::filter_args_for_mode_link(invocation)?;

        // build action
        let action = if is_compile_only {
            if link_libs_opt.is_some() {
                bail!("unexpected linking flags in compile-only mode");
            }
            if inputs.len() != 1 {
                bail!("more than one inputs in compile-only mode ");
            }
            let input = inputs.into_iter().next().unwrap();
            Action::Compile {
                input,
                output,
                invocation,
            }
        } else {
            // at least linking is involved
            let libs = link_libs_opt.unwrap_or_default();

            // now decide whether this is linking only or compile-and-link
            if inputs.len() == 1 {
                let input = inputs.into_iter().next().unwrap();
                let extension = match CommonExtensions::probe(&input) {
                    None => bail!(
                        "unable to guess the action for single-file invocation: {}",
                        input.to_string_lossy()
                    ),
                    Some(e) => e,
                };
                match extension {
                    CommonExtensions::C | CommonExtensions::CPP | CommonExtensions::Asm => {
                        Action::CompileAndLink {
                            input,
                            libs,
                            output,
                            invocation,
                        }
                    }
                    CommonExtensions::Object
                    | CommonExtensions::LibShared
                    | CommonExtensions::LibStatic => Action::Link {
                        inputs: vec![
                            // see below for reasons to canonicalize this input
                            input.canonicalize()?,
                        ],
                        libs,
                        output,
                        invocation,
                    },
                }
            } else {
                // canonicalize all input paths for linking
                // NOTE: this is not required for compilation as we don't care how source code input
                // are obtained (as long as they exist). But we canonicalize paths for libraries as
                // library files can be symlinked (e.g., lib<name>.so -> lib<name>.so.<version>).
                let canonical_inputs = inputs
                    .iter()
                    .map(|e| e.canonicalize())
                    .collect::<io::Result<_>>()?;

                // more than one input, mark it as linking
                Action::Link {
                    inputs: canonical_inputs,
                    libs,
                    output,
                    invocation,
                }
            }
        };

        // done
        Ok(action)
    }
}

impl Action {
    /// Retrieve the output of this action
    pub fn output(&self) -> &Path {
        match self {
            Self::Compile { output, .. }
            | Self::Link { output, .. }
            | Self::CompileAndLink { output, .. } => output,
        }
    }

    /// Invoke the build action for whole-program LLVM
    pub fn invoke_for_wllvm(&self) -> Result<()> {
        // unpack
        let output = self.output();
        debug!("[wllvm] processing: {}", output.to_string_lossy());

        let ClangInvocation { cwd, cxx, args } = match self {
            Self::Compile { invocation, .. }
            | Self::Link { invocation, .. }
            | Self::CompileAndLink { invocation, .. } => invocation,
        };

        let new_ext = output.extension().map_or_else(
            || BITCODE_EXTENSION.to_string(),
            |e| {
                format!(
                    "{}.{}",
                    e.to_str().expect("pure ASCII extension"),
                    BITCODE_EXTENSION
                )
            },
        );
        let bitcode_output = output.with_extension(new_ext);

        // prepare command
        let ctxt = Context::new().expect("LLVM context");
        let name = if *cxx { "clang++" } else { "clang" };
        let bin_clang = ctxt.path_llvm(["bin", name]).expect("ascii path only");

        let mut cmd = Command::new(bin_clang);
        cmd.current_dir(cwd);

        // branch by action type
        match self {
            Self::Compile {
                input,
                output: _,
                invocation: _,
            } => {
                // header
                cmd.arg("-c").arg("-emit-llvm");

                // arguments
                for option in args {
                    match option {
                        // pass through
                        ClangArg::Standard(..)
                        | ClangArg::Define(..)
                        | ClangArg::Include(..)
                        | ClangArg::IncludeSysroot(..)
                        | ClangArg::Arch(..)
                        | ClangArg::MachineArch(..)
                        | ClangArg::Debug
                        | ClangArg::FlagPIC(..)
                        | ClangArg::FlagPIE(..)
                        | ClangArg::FlagRTTI(..)
                        | ClangArg::FlagExceptions(..)
                        | ClangArg::Warning(..)
                        | ClangArg::NoWarnings
                        | ClangArg::Pedantic
                        | ClangArg::POSIXThread => {
                            cmd.args(option.as_args());
                        }
                        // ignored
                        ClangArg::Optimization(..) | ClangArg::PrepMD(..) | ClangArg::Print(..) => {
                        }
                        // unexpected
                        ClangArg::ModeCompile
                        | ClangArg::LibName(..)
                        | ClangArg::LibPath(..)
                        | ClangArg::LinkShared
                        | ClangArg::LinkStatic
                        | ClangArg::LinkRpath(..)
                        | ClangArg::LinkSoname(..)
                        | ClangArg::Output(..)
                        | ClangArg::Input(..) => {
                            bail!("unexpected {} option: {}", name, option)
                        }
                    }
                }

                // input and output
                cmd.arg("-o").arg(bitcode_output);
                cmd.arg(input);
            }
            Self::Link { .. } | Self::CompileAndLink { .. } => todo!(),
        }

        // invoke the command
        let status = cmd.status()?;
        if !status.success() {
            let args: Vec<_> = cmd.get_args().map(|e| e.to_string_lossy()).collect();
            bail!("failed to execute command: {}", args.join(" "));
        }
        Ok(())
    }
}

/// Scan over the directory and collect build commands
pub fn build_database(path_src: &Path) -> Result<()> {
    // collect commands
    let mut actions = BTreeMap::new();
    for entry in WalkDir::new(path_src) {
        let entry = entry?;
        let path = entry.path();
        if path
            .file_name()
            .and_then(|e| e.to_str())
            .map_or(false, |e| e.ends_with(COMMAND_EXTENSION))
        {
            let content = fs::read_to_string(path)?;
            let invocation: ClangInvocation = serde_json::from_str(&content)?;
            let action = Action::parse(invocation)?;
            let exists = actions.insert(action.output().to_path_buf(), action);
            match exists {
                None => (),
                Some(another) => {
                    bail!(
                        "output defined multiple times: {}",
                        another.output().to_string_lossy()
                    );
                }
            }
        }
    }

    // build the compilation graph (DAG)
    let mut graph = DiGraph::new();
    let mut nodes = BTreeMap::new();

    // add nodes
    for key in actions.keys() {
        let nid = graph.add_node(key.to_path_buf());
        nodes.insert(key.to_path_buf(), nid);
    }

    // add edges
    for (key, val) in &actions {
        let dst = *nodes.get(key).unwrap();
        match val {
            Action::Compile { .. } | Action::CompileAndLink { .. } => (),
            Action::Link { inputs, .. } => {
                for item in inputs {
                    let src = match nodes.get(item) {
                        None => bail!("linker input does not exist: {}", item.to_string_lossy()),
                        Some(idx) => *idx,
                    };
                    graph.add_edge(src, dst, ());
                }
            }
        }
    }

    // ensures that the graph is a DAG
    let ordered = match toposort(&graph, None) {
        Ok(nodes) => nodes,
        Err(_) => bail!("expect a DAG in the build graph"),
    };

    // build and merge according to topological order
    for nid in ordered {
        let key = graph.node_weight(nid).unwrap();
        let action = actions.get(key).unwrap();
        action.invoke_for_wllvm()?;
    }

    // done
    Ok(())
}

/// Like `fs::canonicalize`, but without resolving and symbolic links
fn normalize_path<P: AsRef<Path>, Q: AsRef<Path>>(cwd: P, path: Q) -> PathBuf {
    let path = path.as_ref();

    let mut absolute = if path.is_absolute() {
        PathBuf::new()
    } else {
        cwd.as_ref().to_path_buf()
    };
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                absolute.pop();
            }
            component => absolute.push(component.as_os_str()),
        }
    }

    absolute
}
