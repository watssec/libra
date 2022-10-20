use llvm_ir::Module as LLVMModule;

use crate::ir::bridge::module::Module;
use crate::EngineResult;

mod cfg;
mod constant;
mod function;
mod global;
mod instruction;
mod module;
mod shared;
mod typing;

/// Transfer function
pub fn convert(llvm_module: &LLVMModule) -> EngineResult<Module> {
    Module::convert(llvm_module)
}
