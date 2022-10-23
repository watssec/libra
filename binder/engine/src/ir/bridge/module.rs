use crate::error::{EngineError, Unsupported};
use crate::ir::adapter;
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
}

impl Module {
    pub fn convert(prefix: &str, module_adapted: &adapter::module::Module) -> EngineResult<Self> {
        let adapter::module::Module { name, asm, structs } = module_adapted;

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

        // done
        Ok(Self {
            name: ident,
            typing,
        })
    }
}
