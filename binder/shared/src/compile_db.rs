use std::fs;
use std::path::Path;
use std::str::Split;

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct CompileEntry {
    pub file: String,
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

    pub fn next_token_or_end(&mut self) -> Option<&'a str> {
        self.tokens.next()
    }

    pub fn prev_token_or_end(&mut self) -> Option<&'a str> {
        self.tokens.next_back()
    }

    fn expect_token(item: Option<&'a str>) -> Result<&'a str> {
        match item {
            None => bail!("expect <token>, found none"),
            Some(token) => Ok(token),
        }
    }

    pub fn next_expect_token(&mut self) -> Result<&'a str> {
        Self::expect_token(self.tokens.next())
    }

    pub fn prev_expect_token(&mut self) -> Result<&'a str> {
        Self::expect_token(self.tokens.next_back())
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
    /// -O<level>
    Optimization(String),
    /// -arch <token>
    Architecture(String),
    /// -g, --debug
    Debug,
    /// -isysroot <token>
    IncludeSysroot(String),
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
        let arg = match stream.next_token_or_end() {
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
                        "-arch" => {
                            Self::Architecture(Self::expect_plain(stream.next_expect_token()?)?)
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

    pub fn consume(mut stream: TokenStream) -> Result<Vec<Self>> {
        let mut args = vec![];
        while let Some(arg) = Self::try_parse(&mut stream)? {
            args.push(arg);
        }
        Ok(args)
    }
}
