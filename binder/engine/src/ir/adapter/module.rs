use serde::{Deserialize, Serialize};

/// A representation of an LLVM module
#[derive(Serialize, Deserialize)]
pub struct Module {
    /// name of the module
    pub name: String,
    /// module-level assembly
    pub asm: String,
}
