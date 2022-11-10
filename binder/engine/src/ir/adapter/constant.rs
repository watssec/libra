use serde::{Deserialize, Serialize};

use crate::ir::adapter::instruction::Inst;
use crate::ir::adapter::typing::Type;

/// A representation of an LLVM constant
#[derive(Serialize, Deserialize, Clone)]
pub enum Const {
    Int { value: u64 },
    Float { value: String },
    Null,
    None,
    Undef,
    PC,
    Default,
    Array { elements: Vec<Constant> },
    Vector { elements: Vec<Constant> },
    Struct { elements: Vec<Constant> },
    Variable { name: Option<String> },
    Function { name: Option<String> },
    Alias { name: Option<String> },
    Interface { name: Option<String> },
    Expr { inst: Box<Inst> },
}

/// A representation of an LLVM constant
#[derive(Serialize, Deserialize, Clone)]
pub struct Constant {
    /// type of the constant
    pub ty: Type,
    /// the actual representation of a constant
    pub repr: Const,
}
