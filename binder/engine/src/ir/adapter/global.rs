use serde::{Deserialize, Serialize};

use crate::ir::adapter::constant::Constant;
use crate::ir::adapter::typing::Type;

/// An adapted representation of an LLVM global variable
#[derive(Serialize, Deserialize, Clone)]
pub struct GlobalVariable {
    /// variable name
    pub name: Option<String>,
    /// variable type
    pub ty: Type,
    /// is not just a declaration
    pub is_defined: bool,
    /// the definition (initialization) is exact
    pub is_exact: bool,
    /// is constant (immutable) during execution
    pub is_const: bool,
    /// is thread-local (one copy per thread)
    pub is_thread_local: bool,
    /// address space of the global variable
    pub address_space: usize,
    /// initializer
    pub initializer: Option<Constant>,
}
