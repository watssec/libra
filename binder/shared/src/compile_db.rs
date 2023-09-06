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
