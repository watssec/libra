use std::fmt::{Display, Formatter};
use std::fs;
use std::path::{Path, PathBuf};
use std::str::Split;

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct CompileEntry {
    pub file: String,
    #[cfg(target_os = "macos")]
    pub output: String,
    pub directory: String,
    pub command: String,
}

pub struct CompileDB {
    pub entries: Vec<CompileEntry>,
}

impl CompileDB {
    pub fn new(path: &Path) -> Result<Self> {
        if !path.is_file() {
            bail!("unable to locate compilation database");
        }
        let content = fs::read_to_string(path)?;
        let entries: Vec<CompileEntry> = serde_json::from_str(&content)?;
        Ok(Self { entries })
    }
}

pub struct TokenStream<'a> {
    tokens: Split<'a, char>,
}

impl<'a> TokenStream<'a> {
    pub fn new(tokens: Split<'a, char>) -> Self {
        Self { tokens }
    }

    pub fn next_or_end(&mut self) -> Option<&'a str> {
        loop {
            match self.tokens.next() {
                None => return None,
                Some("") => continue,
                Some(v) => return Some(v),
            }
        }
    }

    pub fn prev_or_end(&mut self) -> Option<&'a str> {
        loop {
            match self.tokens.next_back() {
                None => return None,
                Some("") => continue,
                Some(v) => return Some(v),
            }
        }
    }

    fn expect_token(item: Option<&'a str>) -> Result<&'a str> {
        match item {
            None => bail!("expect <token>, found none"),
            Some(token) => Ok(token),
        }
    }

    pub fn next_expect_token(&mut self) -> Result<&'a str> {
        Self::expect_token(self.next_or_end())
    }

    pub fn prev_expect_token(&mut self) -> Result<&'a str> {
        Self::expect_token(self.prev_or_end())
    }

    fn expect_literal(item: Option<&'a str>, exp: &str) -> Result<()> {
        match item {
            None => bail!("expect '{}', found none", exp),
            Some(token) => {
                if token != exp {
                    bail!("expect '{}', found '{}'", exp, token);
                }
            }
        }
        Ok(())
    }

    pub fn next_expect_literal(&mut self, exp: &str) -> Result<()> {
        Self::expect_literal(self.tokens.next(), exp)
    }

    pub fn prev_expect_literal(&mut self, exp: &str) -> Result<()> {
        Self::expect_literal(self.tokens.next_back(), exp)
    }
}

pub enum ClangArg {
    /// -c
    ModeCompile,
    /// -std=<token>
    Standard(String),
    /// -D<token>
    Define(String),
    /// -I <token>
    Include(String),
    /// -isysroot <token>
    IncludeSysroot(String),
    /// -O<level>
    Optimization(String),
    /// -arch <token>
    Arch(String),
    /// -march=<token>
    MachineArch(String),
    #[cfg(target_os = "macos")]
    /// -mmacosx-<key>=<value>, e.g., -mmacosx-version-min=12.4
    MacOSX(String, Option<String>),
    /// -g, --debug
    Debug,
    /// -f<key>{=<value>}
    Flag(String, Option<String>),
    /// -W<key>{=<value>}
    Warning(String, Option<String>),
    /// -w, --no-warnings
    NoWarnings,
    /// -pthread
    POSIXThread,
    /// -o <token>
    Output(String),
    /// <token>
    Input(String),
}

impl ClangArg {
    fn unescape(
        prefix: char,
        suffix: char,
        cur: &str,
        stream: &mut TokenStream,
    ) -> Result<(bool, String)> {
        let mut ptr = match cur.strip_prefix(prefix) {
            None => return Ok((false, cur.to_string())),
            Some(s) => s,
        };

        let mut items = vec![];
        loop {
            match ptr.strip_suffix(suffix) {
                None => {
                    items.push(ptr);
                    ptr = stream.next_expect_token()?;
                }
                Some(s) => {
                    items.push(s);
                    break;
                }
            }
        }

        Ok((true, items.join(" ")))
    }

    fn unescape_double_quotes(cur: &str, stream: &mut TokenStream) -> Result<(bool, String)> {
        Self::unescape('"', '"', cur, stream)
    }

    fn unescape_single_quotes(cur: &str, stream: &mut TokenStream) -> Result<(bool, String)> {
        Self::unescape('\'', '\'', cur, stream)
    }

    fn unescape_quotes(cur: &str, stream: &mut TokenStream) -> Result<String> {
        let (changed, result) = Self::unescape_double_quotes(cur, stream)?;
        if changed {
            Ok(result)
        } else {
            let (_, result) = Self::unescape_single_quotes(cur, stream)?;
            Ok(result)
        }
    }

    fn expect_plain(cur: &str) -> Result<String> {
        if cur.starts_with('"') || cur.starts_with('\'') {
            bail!("unexpected token with quotes: {}", cur);
        }
        Ok(cur.to_string())
    }

    fn parse_maybe_key_value(
        cur: &str,
        stream: &mut TokenStream,
    ) -> Result<(String, Option<String>)> {
        let item = Self::expect_plain(cur)?;
        let result = match item.find('=') {
            None => (item, None),
            Some(index) => {
                let (key, val) = item.split_at(index);
                let val = val.strip_prefix('=').unwrap();
                (key.to_string(), Some(Self::unescape_quotes(val, stream)?))
            }
        };
        Ok(result)
    }

    fn try_parse(stream: &mut TokenStream) -> Result<Option<Self>> {
        let arg = match stream.next_or_end() {
            None => return Ok(None),
            Some(token) => {
                if !token.starts_with('-') {
                    Self::Input(token.to_string())
                } else {
                    match token {
                        "-c" => Self::ModeCompile,
                        t if t.starts_with("-std=") => {
                            let item = t.strip_prefix("-std=").unwrap();
                            Self::Standard(Self::expect_plain(item)?)
                        }
                        t if t.starts_with("-D") => {
                            let item = t.strip_prefix("-D").unwrap();
                            Self::Define(Self::unescape_quotes(item, stream)?)
                        }
                        "-I" => Self::Include(Self::unescape_quotes(
                            stream.next_expect_token()?,
                            stream,
                        )?),
                        t if t.starts_with("-I") => {
                            let item = t.strip_prefix("-I").unwrap();
                            Self::Include(Self::unescape_quotes(item, stream)?)
                        }
                        t if t.starts_with("-O") => {
                            let item = t.strip_prefix("-O").unwrap();
                            Self::Optimization(Self::expect_plain(item)?)
                        }
                        "-arch" => Self::Arch(Self::expect_plain(stream.next_expect_token()?)?),
                        t if t.starts_with("-march=") => {
                            let item = t.strip_prefix("-march=").unwrap();
                            Self::MachineArch(Self::expect_plain(item)?)
                        }
                        #[cfg(target_os = "macos")]
                        t if t.starts_with("-mmacosx-") => {
                            let item = t.strip_prefix("-mmacosx-").unwrap();
                            let (k, v) = Self::parse_maybe_key_value(item, stream)?;
                            Self::MacOSX(k, v)
                        }
                        "-g" | "--debug" => Self::Debug,
                        "-isysroot" => Self::IncludeSysroot(Self::unescape_quotes(
                            stream.next_expect_token()?,
                            stream,
                        )?),
                        t if t.starts_with("-f") => {
                            let item = t.strip_prefix("-f").unwrap();
                            let (k, v) = Self::parse_maybe_key_value(item, stream)?;
                            Self::Flag(k, v)
                        }
                        t if t.starts_with("-W") => {
                            let item = t.strip_prefix("-W").unwrap();
                            let (k, v) = Self::parse_maybe_key_value(item, stream)?;
                            Self::Warning(k, v)
                        }
                        "-w" | "--no-warnings" => Self::NoWarnings,
                        "-pthread" => Self::POSIXThread,
                        "-o" => Self::Output(Self::unescape_quotes(
                            stream.next_expect_token()?,
                            stream,
                        )?),
                        _ => bail!("unknown flag: {}", token),
                    }
                }
            }
        };
        Ok(Some(arg))
    }
}

impl Display for ClangArg {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ModeCompile => write!(f, "-c"),
            Self::Standard(v) => write!(f, "-std={}", v),
            Self::Define(v) => write!(f, "-D{}", v),
            Self::Include(v) => write!(f, "-I{}", v),
            Self::IncludeSysroot(v) => write!(f, "-isysroot {}", v),
            Self::Optimization(v) => write!(f, "-O{}", v),
            Self::Arch(v) => write!(f, "-arch {}", v),
            Self::MachineArch(v) => write!(f, "-march={}", v),
            #[cfg(target_os = "macos")]
            Self::MacOSX(k, None) => write!(f, "-mmacosx-{}", k),
            #[cfg(target_os = "macos")]
            Self::MacOSX(k, Some(v)) => write!(f, "-mmacosx-{}={}", k, v),
            Self::Debug => write!(f, "-g"),
            Self::Flag(k, None) => write!(f, "-f{}", k),
            Self::Flag(k, Some(v)) => write!(f, "-f{}={}", k, v),
            Self::Warning(k, None) => write!(f, "-W{}", k),
            Self::Warning(k, Some(v)) => write!(f, "-W{}={}", k, v),
            Self::NoWarnings => write!(f, "-w"),
            Self::POSIXThread => write!(f, "-pthread"),
            Self::Output(v) => write!(f, "-o {}", v),
            Self::Input(v) => write!(f, "{}", v),
        }
    }
}

impl ClangArg {
    fn accumulate_arg_for_libra(&self, args: &mut Vec<String>) {
        match self {
            Self::ModeCompile => {
                args.push("-c".into());
            }
            Self::Standard(v) => {
                args.push(format!("-std={}", v));
            }
            Self::Define(v) => {
                args.push(format!("-D{}", v));
            }
            Self::Include(v) => {
                args.push(format!("-I{}", v));
            }
            Self::IncludeSysroot(v) => {
                args.push("-isysroot".into());
                args.push(v.to_string());
            }
            Self::Optimization(_) => {
                // NOTE: libra handles optimization itself
            }
            Self::Arch(v) => {
                args.push("-arch".to_string());
                args.push(v.to_string());
            }
            Self::MachineArch(v) => {
                args.push(format!("-march={}", v));
            }
            #[cfg(target_os = "macos")]
            Self::MacOSX(k, None) => {
                args.push(format!("-mmacosx-{}", k));
            }
            #[cfg(target_os = "macos")]
            Self::MacOSX(k, Some(v)) => {
                args.push(format!("-mmacosx-{}={}", k, v));
            }
            Self::Debug => {
                // NOTE: libra handles metadata itself
            }
            Self::Flag(k, None) => {
                args.push(format!("-f{}", k));
            }
            Self::Flag(k, Some(v)) => {
                args.push(format!("-f{}={}", k, v));
            }
            Self::Warning(k, None) => {
                args.push(format!("-W{}", k));
            }
            Self::Warning(k, Some(v)) => {
                args.push(format!("-W{}={}", k, v));
            }
            Self::NoWarnings => {
                args.push("-w".into());
            }
            Self::POSIXThread => {
                args.push("-pthread".into());
            }
            Self::Output(_) => (),
            Self::Input(_) => (),
        }
    }
}

pub struct ClangCommand {
    is_cpp: bool,
    pub workdir: PathBuf,
    args: Vec<ClangArg>,
}

impl ClangCommand {
    pub fn new(is_cpp: bool, workdir: PathBuf, mut stream: TokenStream) -> Result<Self> {
        let mut args = vec![];
        while let Some(arg) = ClangArg::try_parse(&mut stream)? {
            args.push(arg);
        }
        Ok(Self {
            is_cpp,
            workdir,
            args,
        })
    }

    pub fn outputs(&self) -> Vec<&str> {
        self.args
            .iter()
            .filter_map(|arg| match arg {
                ClangArg::Output(v) => Some(v.as_str()),
                _ => None,
            })
            .collect()
    }

    pub fn inputs(&self) -> Vec<&str> {
        self.args
            .iter()
            .filter_map(|arg| match arg {
                ClangArg::Input(v) => Some(v.as_str()),
                _ => None,
            })
            .collect()
    }

    pub fn infer_language(&self) -> Option<ClangSupportedLanguage> {
        // TODO: is this the best way?
        if self.is_cpp {
            return Some(ClangSupportedLanguage::CPP);
        }

        // guess language
        let mut inferred = None;
        for input in self.inputs() {
            let ext = Path::new(input).extension().and_then(|ext| ext.to_str())?;
            let lang = match ext {
                "c" => ClangSupportedLanguage::C,
                "cc" | "cpp" => ClangSupportedLanguage::CPP,
                "m" => ClangSupportedLanguage::ObjC,
                "mm" => ClangSupportedLanguage::ObjCPP,
                "bc" | "ll" => ClangSupportedLanguage::Bitcode,
                "o" => ClangSupportedLanguage::Object,
                _ => {
                    return None;
                }
            };
            // input has mixed languages
            if matches!(inferred, Some(existing) if existing != lang) {
                return None;
            }
            inferred = Some(lang);
        }
        inferred
    }

    pub fn gen_args_for_libra(&self) -> Vec<String> {
        let mut accumulated = vec![];
        for arg in &self.args {
            arg.accumulate_arg_for_libra(&mut accumulated);
        }

        // allow libra to handle optimization on its own
        accumulated.push("-Xclang".to_string());
        accumulated.push("-disable-O0-optnone".to_string());
        accumulated
    }
}

impl Display for ClangCommand {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let Self {
            is_cpp,
            workdir: _,
            args,
        } = self;

        let mut tokens = vec![if *is_cpp { "clang++" } else { "clang" }.to_string()];
        tokens.extend(args.iter().map(|arg| arg.to_string()));
        write!(f, "{}", tokens.join(" "))
    }
}

#[derive(Ord, PartialOrd, Eq, PartialEq)]
pub enum ClangSupportedLanguage {
    /// .c
    C,
    /// .cc, .cpp
    CPP,
    /// .m
    ObjC,
    /// .mm
    ObjCPP,
    /// .ll, .bc
    Bitcode,
    /// .o
    Object,
}
