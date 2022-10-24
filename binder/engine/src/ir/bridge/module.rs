use std::collections::{BTreeMap, BTreeSet};

use crate::error::{EngineError, Unsupported};
use crate::ir::adapter;
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
    /// global variables
    globals: BTreeMap<Identifier, GlobalVariable>,
}

impl Module {
    pub fn convert(prefix: &str, module_adapted: &adapter::module::Module) -> EngineResult<Self> {
        let adapter::module::Module {
            name,
            asm,
            structs,
            global_variables,
        } = module_adapted;

        // check name
        let ident = match name.strip_prefix(prefix) {
            None => {
                return Err(EngineError::InvariantViolation(format!(
                    "module name `{}` does not start with prefix `{}`",
                    name, prefix
                )));
            }
            Some(n) => n.into(),
        };

        // reject module-level inline assembly
        if !asm.is_empty() {
            return Err(EngineError::NotSupportedYet(
                Unsupported::ModuleLevelAssembly,
            ));
        }

        // build type registry
        let typing = TypeRegistry::populate(structs)?;

        // collect global variables
        let allowed_globals: BTreeSet<Identifier> = global_variables
            .iter()
            .filter_map(|gvar| gvar.name.as_ref().map(|e| e.into()))
            .collect();

        let mut globals = BTreeMap::new();
        for gvar in global_variables.iter() {
            let converted = GlobalVariable::convert(gvar, &typing, &allowed_globals)?;
            match globals.insert(converted.name.clone(), converted) {
                None => (),
                Some(_) => {
                    return Err(EngineError::InvalidAssumption(format!(
                        "no duplicated global variable: {}",
                        gvar.name.as_ref().unwrap()
                    )));
                }
            }
        }

        // done
        Ok(Self {
            name: ident,
            typing,
            globals,
        })
    }
}
