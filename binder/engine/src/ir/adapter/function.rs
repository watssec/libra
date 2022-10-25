use serde::{Deserialize, Serialize};

use crate::ir::adapter::typing::Type;

/// A representation of an LLVM function
#[derive(Serialize, Deserialize)]
pub struct Function {
    /// name of the module
    pub name: Option<String>,
    /// type of the function
    pub ty: Type,
    /// is not just a declaration
    pub is_defined: bool,
    /// the definition (function body) is exact
    pub is_exact: bool,
    /// parameters
    pub params: Vec<Parameter>,
    /// intrinsics id (if applicable)
    pub intrinsics: Option<usize>,
}

/// A representation of an LLVM function parameter
#[derive(Serialize, Deserialize)]
pub struct Parameter {
    /// name of the module
    pub name: Option<String>,
    /// type of the function
    pub ty: Type,
}
