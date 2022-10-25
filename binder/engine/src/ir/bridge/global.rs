use crate::error::{EngineError, Unsupported};
use crate::ir::adapter;
use crate::ir::bridge::constant::Constant;
use crate::ir::bridge::shared::{Identifier, SymbolRegistry};
use crate::ir::bridge::typing::{Type, TypeRegistry};
use crate::EngineResult;

/// An adapted representation of an LLVM global variable
#[derive(Eq, PartialEq, Clone, Debug)]
pub struct GlobalVariable {
    /// variable name
    pub name: Identifier,
    /// variable type
    pub ty: Type,
    /// initializer
    pub initializer: Option<Constant>,
}

impl GlobalVariable {
    pub fn convert(
        gvar: &adapter::global::GlobalVariable,
        typing: &TypeRegistry,
        symbols: &SymbolRegistry,
    ) -> EngineResult<Self> {
        let adapter::global::GlobalVariable {
            name,
            ty,
            is_extern,
            is_const,
            is_defined,
            is_exact,
            is_thread_local,
            address_space,
            initializer,
        } = gvar;

        // filter out unsupported cases
        if *is_extern {
            return Err(EngineError::NotSupportedYet(
                Unsupported::ExternGlobalVariable,
            ));
        }
        if !*is_exact {
            return Err(EngineError::NotSupportedYet(
                Unsupported::WeakGlobalVariable,
            ));
        }
        if *is_thread_local {
            return Err(EngineError::NotSupportedYet(
                Unsupported::ThreadLocalStorage,
            ));
        }
        if *address_space != 0 {
            return Err(EngineError::NotSupportedYet(
                Unsupported::PointerAddressSpace,
            ));
        }
        if *is_const && initializer.is_none() {
            return Err(EngineError::InvalidAssumption(format!(
                "must have an initializer for a constant global: {}",
                name.as_ref().map_or("<unknown>", |e| e.as_str())
            )));
        }
        if *is_defined && initializer.is_none() {
            return Err(EngineError::InvalidAssumption(format!(
                "must have an initializer for a defined global: {}",
                name.as_ref().map_or("<unknown>", |e| e.as_str())
            )));
        }

        // convert the name
        let ident: Identifier = name
            .as_ref()
            .ok_or_else(|| EngineError::InvalidAssumption("no anonymous global variable".into()))?
            .into();

        // convert the type
        let gvar_ty = typing.convert(ty)?;

        // convert the initializer (if any)
        let gvar_init = match initializer {
            None => None,
            Some(constant) => Some(Constant::convert(constant, &gvar_ty, typing, symbols)?),
        };

        // done with the construction
        Ok(Self {
            name: ident,
            ty: gvar_ty,
            initializer: gvar_init,
        })
    }
}
