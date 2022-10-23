use crate::error::{EngineError, Unsupported};
use crate::ir::adapter;
use crate::ir::bridge::shared::Identifier;
use crate::ir::bridge::typing::{Type, TypeRegistry};
use crate::EngineResult;

/// An adapted representation of an LLVM global variable
#[derive(Eq, PartialEq, Clone, Debug)]
pub struct GlobalVariable {
    /// variable name
    pub name: Identifier,
    /// variable type
    pub ty: Type,
    // TODO initializer
    //pub initializer: Option<Constant>,
}

impl GlobalVariable {
    pub fn convert(
        gvar: &adapter::global::GlobalVariable,
        typing: &TypeRegistry,
    ) -> EngineResult<Self> {
        let adapter::global::GlobalVariable {
            name,
            ty,
            is_extern,
            is_const: _,
            is_exact,
            is_thread_local,
            address_space,
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

        // convert the name
        let ident: Identifier = name
            .as_ref()
            .ok_or_else(|| EngineError::InvalidAssumption("no anonymous global variable".into()))?
            .into();

        // convert the type
        let gvar_ty = typing.convert(ty)?;

        // TODO: convert constant
        // TODO: check that all immutable globals have initializer

        // done with the construction
        Ok(Self {
            name: ident,
            ty: gvar_ty,
        })
    }
}
