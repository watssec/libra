use std::collections::{BTreeMap, BTreeSet};

use log::debug;

use crate::error::{EngineError, EngineResult, Unsupported};
use crate::ir::adapter;
use crate::ir::bridge::function::Function;
use crate::ir::bridge::global::GlobalVariable;
use crate::ir::bridge::shared::{Identifier, SymbolRegistry};
use crate::ir::bridge::typing::TypeRegistry;

/// An adapted representation of an LLVM module
#[derive(Eq, PartialEq)]
pub struct Module {
    /// type registry
    typing: TypeRegistry,
    /// symbol registry
    symbols: SymbolRegistry,
    /// global variables
    globals: BTreeMap<Identifier, GlobalVariable>,
    /// functions
    functions: BTreeMap<Identifier, Function>,
}

impl Module {
    pub fn convert(module_adapted: &adapter::module::Module) -> EngineResult<Self> {
        let adapter::module::Module {
            name,
            asm,
            structs,
            global_variables,
            functions,
        } = module_adapted;

        // check name
        debug!("converting module: {}", name);

        // reject module-level inline assembly
        if !asm.is_empty() {
            return Err(EngineError::NotSupportedYet(
                Unsupported::ModuleLevelAssembly,
            ));
        }

        // build type registry
        let typing = TypeRegistry::populate(structs)?;

        // build symbol registry
        let allowed_globals: BTreeSet<Identifier> = global_variables
            .iter()
            .filter_map(|gvar| gvar.name.as_ref().map(|e| e.into()))
            .collect();
        let allowed_functions: BTreeSet<Identifier> = functions
            .iter()
            .filter_map(|func| func.name.as_ref().map(|e| e.into()))
            .collect();
        let symbols = SymbolRegistry::new(allowed_globals, allowed_functions);

        // collect global variables
        let mut gvar_table = BTreeMap::new();
        for gvar in global_variables.iter() {
            let converted = GlobalVariable::convert(gvar, &typing, &symbols)?;
            match gvar_table.insert(converted.name.clone(), converted) {
                None => (),
                Some(_) => {
                    return Err(EngineError::InvalidAssumption(format!(
                        "no duplicated global variable: {}",
                        gvar.name.as_ref().unwrap()
                    )));
                }
            }
        }

        // collect functions
        let mut func_table = BTreeMap::new();
        for func in functions.iter() {
            let converted = Function::convert(func, &typing, &symbols)?;
            match func_table.insert(converted.name.clone(), converted) {
                None => (),
                Some(_) => {
                    return Err(EngineError::InvalidAssumption(format!(
                        "no duplicated function: {}",
                        func.name.as_ref().unwrap()
                    )));
                }
            }
        }

        // done
        Ok(Self {
            typing,
            symbols,
            globals: gvar_table,
            functions: func_table,
        })
    }
}
