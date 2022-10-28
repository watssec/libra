use serde::{Deserialize, Serialize};

use crate::ir::adapter::typing::Type;

/// A representation of an LLVM constant
#[derive(Serialize, Deserialize)]
pub enum Constant {
    Int { ty: Type, value: u64 },
    Float { ty: Type, value: String },
    Null { ty: Type },
    None { ty: Type },
    Undef { ty: Type },
    Default { ty: Type },
    Array { ty: Type, elements: Vec<Constant> },
    Vector { ty: Type, elements: Vec<Constant> },
    Struct { ty: Type, elements: Vec<Constant> },
    Variable { ty: Type, name: Option<String> },
    Function { ty: Type, name: Option<String> },
    Alias { ty: Type, name: Option<String> },
    Interface { ty: Type, name: Option<String> },
    // TODO: constant expr
}
