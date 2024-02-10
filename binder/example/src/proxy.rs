use std::process::Command;

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

/// Extension for our own command database
pub static COMMAND_EXTENSION: &str = ".command.json";

/// Clang arguments
#[derive(Serialize, Deserialize)]
pub enum ClangArg {
    /// -c
    ModeCompile,
    /// -std=<token>
    Standard(String),
    /// -D<key>{=<value>}
    Define(String, Option<String>),
    /// -I<token>, -I <token>
    Include(String),
    /// -isysroot <token>
    IncludeSysroot(String),
    /// -l<token>, -l <token>
    LibName(String),
    /// -L<token>, -L <token>
    LibPath(String),
    /// -O<level>
    Optimization(String),
    /// -arch <token>
    Arch(String),
    /// -march=<token>
    MachineArch(String),
    /// -g, --debug
    Debug,
    /// -shared, --shared
    LinkShared,
    /// -static, --static
    LinkStatic,
    /// -mllvm -<key>{=<value>}
    Backend(String, Option<String>),
    /// -f<key>{=<value>}
    Flag(String, Option<String>),
    /// -W<key>{=<value>}
    Warning(String, Option<String>),
    /// -w, --no-warnings
    NoWarnings,
    /// -pthread
    POSIXThread,
    /// -print-<key>{=<value>}, --print-<key>{=<value>}
    Print(String, Option<String>),
    /// -pedantic
    Pedantic,
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
            args.push(Self::parse(token, &mut iter));
        }
        args
    }

    fn parse<'a, I>(token: &'a str, stream: &mut I) -> Self
    where
        I: Iterator<Item = &'a str>,
    {
        if !token.starts_with('-') {
            return Self::Input(token.to_string());
        }

        match token {
            "-c" => {
                return Self::ModeCompile;
            }
            "-I" => {
                return Self::Include(Self::expect_next(stream));
            }
            "-l" => {
                return Self::LibName(Self::expect_next(stream));
            }
            "-L" => {
                return Self::LibPath(Self::expect_next(stream));
            }
            "-arch" => {
                return Self::Arch(Self::expect_next(stream));
            }
            "-g" | "--debug" => {
                return Self::Debug;
            }
            "-shared" | "--shared" => {
                return Self::LinkShared;
            }
            "-static" | "--static" => {
                return Self::LinkStatic;
            }
            "-isysroot" => {
                return Self::IncludeSysroot(Self::expect_next(stream));
            }
            "-mllvm" => {
                let (k, v) = Self::expect_maybe_key_value(&Self::expect_next(stream));
                return Self::Backend(k, v);
            }
            "-w" | "--no-warnings" => {
                return Self::NoWarnings;
            }
            "-pthread" => {
                return Self::POSIXThread;
            }
            "-pedantic" => {
                return Self::Pedantic;
            }
            "-o" => {
                return Self::Output(Self::expect_next(stream));
            }
            _ => (),
        }

        if let Some(inner) = token.strip_prefix("-std=") {
            return Self::Standard(inner.to_string());
        }
        if let Some(inner) = token.strip_prefix("-D") {
            let (k, v) = Self::expect_maybe_key_value(inner);
            return Self::Define(k, v);
        }
        if let Some(inner) = token.strip_prefix("-I") {
            return Self::Include(inner.to_string());
        }
        if let Some(inner) = token.strip_prefix("-l") {
            return Self::LibName(inner.to_string());
        }
        if let Some(inner) = token.strip_prefix("-L") {
            return Self::LibPath(inner.to_string());
        }
        if let Some(inner) = token.strip_prefix("-O") {
            return Self::Optimization(inner.to_string());
        }
        if let Some(inner) = token.strip_prefix("-march=") {
            return Self::MachineArch(inner.to_string());
        }
        if let Some(inner) = token.strip_prefix("-f") {
            let (k, v) = Self::expect_maybe_key_value(inner);
            return Self::Flag(k, v);
        }
        if let Some(inner) = token.strip_prefix("-W") {
            let (k, v) = Self::expect_maybe_key_value(inner);
            return Self::Warning(k, v);
        }
        if let Some(inner) = token
            .strip_prefix("-print-")
            .or_else(|| token.strip_prefix("--print-"))
        {
            let (k, v) = Self::expect_maybe_key_value(inner);
            return Self::Print(k, v);
        }

        panic!("unknown Clang option: {}", token);
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
    fn as_cmd_arg(&self, cmd: &mut Command) -> Result<()> {
        match self {
            Self::ModeCompile => bail!("unexpected -c"),
            Self::Standard(val) => cmd.arg(format!("-std={}", val)),
            Self::Define(key, None) => cmd.arg(format!("-D{}", key)),
            Self::Define(key, Some(val)) => cmd.arg(format!("-D{}={}", key, val)),
            Self::Include(val) => cmd.arg(format!("-I{}", val)),
            Self::IncludeSysroot(val) => cmd.arg("-isysroot").arg(val),
            Self::LibName(val) => cmd.arg(format!("-l{}", val)),
            Self::LibPath(val) => cmd.arg(format!("-L{}", val)),
            Self::Optimization(val) => cmd.arg(format!("-O{}", val)),
            Self::Arch(val) => cmd.arg("-arch").arg(val),
            Self::MachineArch(val) => cmd.arg(format!("-march={}", val)),
            Self::Debug => cmd.arg("-g"),
            Self::LinkShared => bail!("unexpected -shared"),
            Self::LinkStatic => bail!("unexpected -static"),
            Self::Backend(key, None) => cmd.arg("-mllvm").arg(key),
            Self::Backend(key, Some(val)) => cmd.arg("-mllvm").arg(format!("{}={}", key, val)),
            Self::Flag(key, None) => cmd.arg(format!("-f{}", key)),
            Self::Flag(key, Some(val)) => cmd.arg(format!("-f{}={}", key, val)),
            Self::Warning(key, None) => cmd.arg(format!("-W{}", key)),
            Self::Warning(key, Some(val)) => cmd.arg(format!("-W{}={}", key, val)),
            Self::NoWarnings => cmd.arg("-w"),
            Self::POSIXThread => cmd.arg("-pthread"),
            Self::Print(key, None) => bail!("unexpected -print-{}", key),
            Self::Print(key, Some(val)) => bail!("unexpected -print-{}={}", key, val),
            Self::Pedantic => cmd.arg("-pedantic"),
            Self::Output(val) => bail!("unexpected -o {}", val),
            Self::Input(val) => bail!("unexpected input {}", val),
        };
        Ok(())
    }
}
