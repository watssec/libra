use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Result};
use petgraph::algo::toposort;
use petgraph::graph::DiGraph;
use walkdir::WalkDir;

use crate::proxy::{ClangArg, ClangInvocation, COMMAND_EXTENSION, LIBMARK_EXTENSION};

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
}

impl CommonExtensions {
    pub fn probe(path: &Path) -> Option<Self> {
        let lang = match path.extension().and_then(|e| e.to_str())? {
            "c" => Self::C,
            "cpp" => Self::CPP,
            "cc" => Self::CPP,
            "s" => Self::Asm,
            "o" => Self::Object,
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
        let mut target = None;
        for item in args {
            if let ClangArg::Output(name) = &item {
                if target.is_some() {
                    panic!("more than one output specified");
                }

                // resolve path
                let path = Path::new(name);
                let path_resolved = if path.is_absolute() {
                    path.to_path_buf()
                } else {
                    cwd.join(path)
                };
                if !path_resolved.exists() {
                    bail!("output path does not exist");
                }
                target = Some(path_resolved.canonicalize()?);
            } else {
                new_args.push(item);
            }
        }

        let output = match target {
            None => bail!("no output in the invocation"),
            Some(out) => out,
        };
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
        let mut inputs = vec![];
        for item in args {
            if let ClangArg::Input(name) = &item {
                // resolve path
                let path = Path::new(name);
                let path_resolved = if path.is_absolute() {
                    path.to_path_buf()
                } else {
                    cwd.join(path)
                };
                if !path_resolved.exists() {
                    bail!("input path does not exist");
                }
                inputs.push(path_resolved.canonicalize()?);
            } else {
                new_args.push(item);
            }
        }

        if inputs.is_empty() {
            bail!("no inputs in the invocation");
        }
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

        let mut is_compile_only = false;
        let mut new_args = vec![];
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

        // collect libraries
        let mut has_linking_flags = false;
        let mut lib_names = vec![];
        let mut lib_paths = vec![];
        let mut libs_sys = vec![];
        let mut new_args = vec![];
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
                ClangArg::LinkStatic | ClangArg::LinkShared | ClangArg::Linker(..) => {
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
                        // compile and link mode
                        Action::CompileAndLink {
                            input,
                            libs,
                            output,
                            invocation,
                        }
                    }
                    CommonExtensions::Object => {
                        // linking mode
                        Action::Link {
                            inputs: vec![input],
                            libs,
                            output,
                            invocation,
                        }
                    }
                }
            } else {
                Action::Link {
                    inputs,
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
    pub fn output(&self) -> &Path {
        match self {
            Self::Compile { output, .. }
            | Self::Link { output, .. }
            | Self::CompileAndLink { output, .. } => output,
        }
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

    // ensures the graph is a DAG
    let ordered = match toposort(&graph, None) {
        Ok(nodes) => nodes,
        Err(_) => bail!("expect a DAG in the build graph"),
    };

    Ok(())
}
