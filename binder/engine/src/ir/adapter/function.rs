use serde::{Deserialize, Serialize};

use crate::ir::adapter::cfg::Block;
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
    /// whether the function is intrinsic
    pub is_intrinsic: bool,
    /// parameters
    pub params: Vec<Parameter>,
    /// body of the function
    pub blocks: Vec<Block>,
}

/// A representation of an LLVM function parameter
#[derive(Serialize, Deserialize)]
pub struct Parameter {
    /// name of the module
    pub name: Option<String>,
    /// type of the function
    pub ty: Type,
    /// attribute: by-val
    pub by_val: Option<Type>,
    /// attribute: by-ref
    pub by_ref: Option<Type>,
    /// attribute: in-alloca
    pub in_alloca: Option<Type>,
    /// attribute: struct-ret
    pub struct_ret: Option<Type>,
    /// attribute: pre-allocated
    pub pre_allocated: Option<Type>,
    /// opaque pointer
    pub element_type: Option<Type>,
}
