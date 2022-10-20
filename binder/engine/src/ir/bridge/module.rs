use std::collections::BTreeMap;

use llvm_ir::Module as LLVMModule;

use crate::error::{EngineError, Unsupported};
use crate::ir::bridge::function::Function;
use crate::ir::bridge::global::GlobalVariable;
use crate::ir::bridge::shared::Identifier;
use crate::ir::bridge::typing::TypeRegistry;
use crate::EngineResult;

/// An adapted representation of an LLVM module
#[derive(Eq, PartialEq)]
pub struct Module {
    /// module name
    name: Identifier,
    /// type registry
    typing: TypeRegistry,
    /// map of global variables
    globals: BTreeMap<Identifier, GlobalVariable>,
    /// map of defined functions
    functions: BTreeMap<Identifier, Function>,
}

impl Module {
    pub fn convert(llvm_module: &LLVMModule) -> EngineResult<Self> {
        // reject module-level inline assembly
        if !llvm_module.inline_assembly.is_empty() {
            return Err(EngineError::NotSupportedYet(
                Unsupported::ModuleLevelAssembly,
            ));
        }

        // build type registry by collecting user-defined structs
        let typing = TypeRegistry::populate(llvm_module)?;

        // reject any global alias
        if !llvm_module.global_aliases.is_empty() {
            return Err(EngineError::NotSupportedYet(
                Unsupported::ModuleLevelAssembly,
            ));
        }

        // collect global variables
        let mut globals = BTreeMap::new();
        for llvm_gvar in llvm_module.global_vars.iter() {
            let converted = GlobalVariable::convert(llvm_module, llvm_gvar, &typing)?;
            match globals.insert(converted.name.clone(), converted) {
                None => (),
                Some(_) => {
                    return Err(EngineError::InvalidAssumption(format!(
                        "no duplicated global variable: {}",
                        llvm_gvar.name
                    )));
                }
            }
        }

        // collect functions
        let mut functions = BTreeMap::new();
        for llvm_fun in llvm_module.functions.iter() {
            let converted = Function::convert(llvm_module, llvm_fun, &typing)?;
            match functions.insert(converted.name.clone(), converted) {
                None => (),
                Some(_) => {
                    return Err(EngineError::InvalidAssumption(format!(
                        "no duplicated function definition: {}",
                        llvm_fun.name
                    )));
                }
            }
        }

        // done with the construction
        Ok(Self {
            name: llvm_module.name.as_str().into(),
            typing,
            globals,
            functions,
        })
    }
}
