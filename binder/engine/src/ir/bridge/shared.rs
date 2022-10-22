use std::fmt::{Display, Formatter};

/// Represents an identifier in the LLVM system
#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Debug)]
pub struct Identifier(String);

impl Display for Identifier {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl From<String> for Identifier {
    fn from(name: String) -> Self {
        Self(name)
    }
}
impl From<&String> for Identifier {
    fn from(name: &String) -> Self {
        Self(name.clone())
    }
}
impl From<&str> for Identifier {
    fn from(name: &str) -> Self {
        Self(name.to_string())
    }
}

impl AsRef<str> for Identifier {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}
