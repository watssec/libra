use std::path::PathBuf;
use std::process::Command;
use std::{env, fmt, fs, process};

use serde::{Deserialize, Serialize};

use libra_engine::flow::shared::Context;

/// Extension for our own command database
pub static COMMAND_EXTENSION: &str = ".command.json";

/// Extension for our own library mark
pub static LIBMARK_EXTENSION: &str = ".library.mark";

/// Clang arguments
#[derive(Serialize, Deserialize)]
pub enum ClangArg {
    /// -c
    ModeCompile,
    /// -std=<token>
    Standard(String),
    /// -D<key>{=<value>}
    Define(String, Option<String>),
    /// -I<token> | -I <token>
    Include(String),
    /// -isysroot <token>
    IncludeSysroot(String),
    /// -Wp,-MD
    PrepMD,
    /// -Wp,-MP
    PrepMP,
    /// -Wp,-MF
    PrepMF(String),
    /// -O<level>
    Optimization(String),
    /// -arch <token>
    Arch(String),
    /// -march=<token>
    MachineArch(String),
    /// -g | --debug
    Debug,
    /// -l<token> | -l <token>
    LibName(String),
    /// -L<token> | -L <token>
    LibPath(String),
    /// -shared | --shared
    LinkShared,
    /// -static | --static
    LinkStatic,
    /// -Wl,-rpath,<token>
    LinkRpath(String),
    /// -Wl,-soname,<token>
    LinkSoname(String),
    /// -Wl,--version-script,<token>
    LinkVersionScript(String),
    /// -fPIC % -fno-PIC
    FlagPIC(bool),
    /// -fPIE % -fno-PIE
    FlagPIE(bool),
    /// -frtti % -fno-rtti
    FlagRTTI(bool),
    /// -fexceptions % -fno-exceptions
    FlagExceptions(bool),
    /// -W<key>{=<value>}
    Warning(String, Option<String>),
    /// -w | --no-warnings
    NoWarnings,
    /// -pedantic
    Pedantic,
    /// -pthread
    POSIXThread,
    /// -print-<key>{=<value>} | --print-<key>{=<value>}
    Print(String, Option<String>),
    /// -o <token>
    Output(String),
    /// <token>
    Input(String),
}

impl ClangArg {
    pub fn collect<'a, I>(mut iter: I) -> Vec<Self>
    where
        I: Iterator<Item = &'a str>,
    {
        let mut args = vec![];
        while let Some(token) = iter.next() {
            args.extend(Self::parse(token, &mut iter));
        }
        args
    }

    fn parse<'a, I>(token: &'a str, stream: &mut I) -> Vec<Self>
    where
        I: Iterator<Item = &'a str>,
    {
        if !token.starts_with('-') {
            return vec![Self::Input(token.to_string())];
        }

        match token {
            "-c" => {
                return vec![Self::ModeCompile];
            }
            "-I" => {
                return vec![Self::Include(Self::expect_next(stream))];
            }
            "-isysroot" => {
                return vec![Self::IncludeSysroot(Self::expect_next(stream))];
            }
            "-l" => {
                return vec![Self::LibName(Self::expect_next(stream))];
            }
            "-L" => {
                return vec![Self::LibPath(Self::expect_next(stream))];
            }
            "-arch" => {
                return vec![Self::Arch(Self::expect_next(stream))];
            }
            "-g" | "--debug" => {
                return vec![Self::Debug];
            }
            "-shared" | "--shared" => {
                return vec![Self::LinkShared];
            }
            "-static" | "--static" => {
                return vec![Self::LinkStatic];
            }
            "-fPIC" => {
                return vec![Self::FlagPIC(true)];
            }
            "-fno-PIC" => {
                return vec![Self::FlagPIC(false)];
            }
            "-fPIE" => {
                return vec![Self::FlagPIE(true)];
            }
            "-fno-PIE" => {
                return vec![Self::FlagPIE(false)];
            }
            "-frtti" => {
                return vec![Self::FlagRTTI(true)];
            }
            "-fno-rtti" => {
                return vec![Self::FlagRTTI(false)];
            }
            "-fexceptions" => {
                return vec![Self::FlagExceptions(true)];
            }
            "-fno-exceptions" => {
                return vec![Self::FlagExceptions(false)];
            }
            "-w" | "--no-warnings" => {
                return vec![Self::NoWarnings];
            }
            "-pedantic" => {
                return vec![Self::Pedantic];
            }
            "-pthread" => {
                return vec![Self::POSIXThread];
            }
            "-o" => {
                return vec![Self::Output(Self::expect_next(stream))];
            }
            _ => (),
        }

        // preprocessor
        if let Some(inner) = token.strip_prefix("-Wp,") {
            if inner.contains('"') || inner.contains('\'') {
                panic!("unexpected quotation marks in {}", token);
            }
            let mut sub_iter = inner.split(",");

            // sub-parser
            let mut args = vec![];
            while let Some(sub_token) = sub_iter.next() {
                args.extend(Self::parse_preprocessor(sub_token, &mut sub_iter));
            }
            return args;
        }

        // linker
        if let Some(inner) = token.strip_prefix("-Wl,") {
            if inner.contains('"') || inner.contains('\'') {
                panic!("unexpected quotation marks in {}", token);
            }
            let mut sub_iter = inner.split(",");

            // sub-parser
            let mut args = vec![];
            while let Some(sub_token) = sub_iter.next() {
                args.extend(Self::parse_linker(sub_token, &mut sub_iter));
            }
            return args;
        }

        // normal
        if let Some(inner) = token.strip_prefix("-std=") {
            return vec![Self::Standard(inner.to_string())];
        }
        if let Some(inner) = token.strip_prefix("-D") {
            let (k, v) = Self::expect_maybe_key_value(inner);
            return vec![Self::Define(k, v)];
        }
        if let Some(inner) = token.strip_prefix("-I") {
            return vec![Self::Include(inner.to_string())];
        }
        if let Some(inner) = token.strip_prefix("-O") {
            return vec![Self::Optimization(inner.to_string())];
        }
        if let Some(inner) = token.strip_prefix("-march=") {
            return vec![Self::MachineArch(inner.to_string())];
        }
        if let Some(inner) = token.strip_prefix("-l") {
            return vec![Self::LibName(inner.to_string())];
        }
        if let Some(inner) = token.strip_prefix("-L") {
            return vec![Self::LibPath(inner.to_string())];
        }
        if let Some(inner) = token.strip_prefix("-W") {
            let (k, v) = Self::expect_maybe_key_value(inner);
            return vec![Self::Warning(k, v)];
        }
        if let Some(inner) = token
            .strip_prefix("-print-")
            .or_else(|| token.strip_prefix("--print-"))
        {
            let (k, v) = Self::expect_maybe_key_value(inner);
            return vec![Self::Print(k, v)];
        }

        panic!("unknown Clang option: {}", token);
    }

    fn parse_preprocessor<'a, I>(token: &'a str, stream: &mut I) -> Vec<Self>
    where
        I: Iterator<Item = &'a str>,
    {
        match token {
            "-MD" => {
                return vec![Self::PrepMD];
            }
            "-MP" => {
                return vec![Self::PrepMP];
            }
            "-MF" => {
                return vec![Self::PrepMF(Self::expect_next(stream))];
            }
            _ => (),
        }

        panic!("unknown Clang option for preprocessor: {}", token);
    }

    fn parse_linker<'a, I>(token: &'a str, stream: &mut I) -> Vec<Self>
    where
        I: Iterator<Item = &'a str>,
    {
        match token {
            "-rpath" => {
                return vec![Self::LinkRpath(Self::expect_next(stream))];
            }
            "-soname" => {
                return vec![Self::LinkSoname(Self::expect_next(stream))];
            }
            "--version-script" => {
                return vec![Self::LinkVersionScript(Self::expect_next(stream))];
            }
            _ => (),
        }

        panic!("unknown Clang option for linker: {}", token);
    }

    fn expect_next<'a, I>(stream: &mut I) -> String
    where
        I: Iterator<Item = &'a str>,
    {
        stream.next().expect("token").to_string()
    }

    fn expect_maybe_key_value(item: &str) -> (String, Option<String>) {
        match item.find('=') {
            None => (item.to_string(), None),
            Some(index) => {
                let (key, val) = item.split_at(index);
                let val = val.strip_prefix('=').unwrap();
                (key.to_string(), Some(val.to_string()))
            }
        }
    }
}

impl ClangArg {
    pub fn as_args(&self) -> Vec<String> {
        match self {
            Self::ModeCompile => vec!["-c".into()],
            Self::Standard(val) => vec![format!("-std={}", val)],
            Self::Define(key, None) => vec![format!("-D{}", key)],
            Self::Define(key, Some(val)) => vec![format!("-D{}={}", key, val)],
            Self::Include(val) => vec![format!("-I{}", val)],
            Self::IncludeSysroot(val) => vec!["-isysroot".into(), val.into()],
            Self::PrepMD => vec!["-Wp,-MD".into()],
            Self::PrepMP => vec!["-Wp,-MP".into()],
            Self::PrepMF(val) => vec![format!("-Wp,-MF,{}", val)],
            Self::Optimization(val) => vec![format!("-O{}", val)],
            Self::Arch(val) => vec!["-arch".into(), val.into()],
            Self::MachineArch(val) => vec![format!("-march={}", val)],
            Self::Debug => vec!["-g".into()],
            Self::LibName(val) => vec![format!("-l{}", val)],
            Self::LibPath(val) => vec![format!("-L{}", val)],
            Self::LinkShared => vec!["-shared".into()],
            Self::LinkStatic => vec!["-static".into()],
            Self::LinkRpath(val) => vec![format!("-Wl,-rpath,{}", val)],
            Self::LinkSoname(val) => vec![format!("-Wl,-soname,{}", val)],
            Self::LinkVersionScript(val) => vec![format!("-Wl,--version-script,{}", val)],
            Self::FlagPIC(true) => vec!["-fPIC".into()],
            Self::FlagPIC(false) => vec!["-fno-PIC".into()],
            Self::FlagPIE(true) => vec!["-fPIE".into()],
            Self::FlagPIE(false) => vec!["-fno-PIE".into()],
            Self::FlagRTTI(true) => vec!["-frtti".into()],
            Self::FlagRTTI(false) => vec!["-fno-rtti".into()],
            Self::FlagExceptions(true) => vec!["-fexceptions".into()],
            Self::FlagExceptions(false) => vec!["-fno-exceptions".into()],
            Self::Warning(key, None) => vec![format!("-W{}", key)],
            Self::Warning(key, Some(val)) => vec![format!("-W{}={}", key, val)],
            Self::NoWarnings => vec!["-w".into()],
            Self::Pedantic => vec!["-pedantic".into()],
            Self::POSIXThread => vec!["-pthread".into()],
            Self::Print(key, None) => vec![format!("-print-{}", key)],
            Self::Print(key, Some(val)) => vec![format!("-print-{}={}", key, val)],
            Self::Output(val) => vec![format!("-o {}", val)],
            Self::Input(val) => vec![format!("unexpected input {}", val)],
        }
    }
}

impl fmt::Display for ClangArg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "{}", self.as_args().join(" "))
    }
}

/// Clang invocation
#[derive(Serialize, Deserialize)]
pub struct ClangInvocation {
    pub cwd: PathBuf,
    pub cxx: bool,
    pub args: Vec<ClangArg>,
}

impl fmt::Display for ClangInvocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        let name = if self.cxx { "clang++" } else { "clang" };
        let mut all_args = vec![];
        for arg in &self.args {
            all_args.extend(arg.as_args());
        }
        write!(f, "{} {}", name, all_args.join(" "))
    }
}

/// Wrap a clang tool
pub fn proxy_clang(cxx: bool) {
    // get paths
    let ctxt = Context::new().expect("LLVM context");
    let name = if cxx { "clang++" } else { "clang" };
    let bin_clang = ctxt.path_llvm(["bin", name]).expect("ascii path only");

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
        cxx,
        args: parsed,
    };

    // serialize
    let content = serde_json::to_string_pretty(&invocation).expect("serialization error");
    fs::write(path, content).expect("IO error");
}
