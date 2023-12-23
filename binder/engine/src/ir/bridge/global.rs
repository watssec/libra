use crate::error::{EngineError, EngineResult, Unsupported};
use crate::ir::adapter;
use crate::ir::bridge::constant::Constant;
use crate::ir::bridge::shared::{Identifier, SymbolRegistry};
use crate::ir::bridge::typing::{Type, TypeRegistry};

/// An adapted representation of an LLVM global variable
#[derive(Eq, PartialEq, Clone)]
pub struct GlobalVariable {
    /// variable name
    pub name: Identifier,
    /// variable type
    pub ty: Type,
    /// mutability
    pub is_constant: bool,
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
            is_const,
            is_defined,
            is_exact,
            is_thread_local,
            address_space,
            initializer,
        } = gvar;

        // filter out unsupported cases
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

        // convert the name
        let ident: Identifier = name
            .as_ref()
            .ok_or(EngineError::NotSupportedYet(
                Unsupported::AnonymousGlobalVariable,
            ))?
            .into();

        // convert the type
        let gvar_ty = typing.convert(ty)?;

        // convert the initializer (if any)
        let gvar_init = match initializer {
            None => {
                if *is_defined {
                    return Err(EngineError::InvalidAssumption(format!(
                        "defined global must have an initializer: {}",
                        ident
                    )));
                }
                None
            }
            Some(constant) => {
                if !*is_defined {
                    return Err(EngineError::InvalidAssumption(format!(
                        "initializer found for an undefined global: {}",
                        ident
                    )));
                }
                Some(Constant::convert(constant, &gvar_ty, typing, symbols)?)
            }
        };

        // done with the construction
        Ok(Self {
            name: ident,
            ty: gvar_ty,
            is_constant: *is_const,
            initializer: gvar_init,
        })
    }
}
