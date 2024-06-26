use serde::{Deserialize, Serialize};

use crate::ir::adapter::function::Function;
use crate::ir::adapter::global::GlobalVariable;
use crate::ir::adapter::typing::UserDefinedStruct;

/// A representation of an LLVM module
#[derive(Serialize, Deserialize)]
pub struct Module {
    /// name of the module
    pub name: String,
    /// module-level assembly
    pub asm: String,
    /// user-defined structs
    pub structs: Vec<UserDefinedStruct>,
    /// global variables
    pub global_variables: Vec<GlobalVariable>,
    /// functions
    pub functions: Vec<Function>,
}
