use serde::{Deserialize, Serialize};

use crate::ir::adapter::typing::Type;

/// An adapted representation of an LLVM global variable
#[derive(Serialize, Deserialize)]
pub struct GlobalVariable {
    /// variable name
    pub name: Option<String>,
    /// variable type
    pub ty: Type,
    /// is externally initialized
    pub is_extern: bool,
    /// is constant (immutable) during execution
    pub is_const: bool,
    /// the definition (initialization) is exact
    pub is_exact: bool,
    /// is thread-local (one copy per thread)
    pub is_thread_local: bool,
    /// address space of the global variable
    pub address_space: usize,
    // TODO: initializer
    //pub initializer: Option<Constant>,
}
