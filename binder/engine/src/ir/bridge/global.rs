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
    /// one-definition rule (ODR)
    pub is_weak: bool,
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
            is_weak: !*is_exact,
            is_constant: *is_const,
            initializer: gvar_init,
        })
    }

    /// Apply the one definition rule
    pub fn apply_odr(entries: Vec<Self>) -> EngineResult<Self> {
        // obtain the strongly defined symbol
        let mut def = None;
        let mut weak_defs = vec![];
        for entry in entries {
            if entry.is_weak {
                weak_defs.push(entry);
                continue;
            }
            if def.is_some() {
                return Err(EngineError::InvalidAssumption(format!(
                    "no duplicated global variable: {}",
                    entry.name
                )));
            }
            def = Some(entry);
        }
        if let Some(v) = def {
            return Ok(v);
        }

        // no strongly defined symbol found, try to unify weak symbols
        let mut iter = weak_defs.into_iter();
        let val = match iter.next() {
            None => {
                return Err(EngineError::InvariantViolation("no entries for ODR".into()));
            }
            Some(v) => v,
        };
        for entry in iter.by_ref() {
            if entry != val {
                return Err(EngineError::NotSupportedYet(
                    Unsupported::WeakGlobalVariable,
                ));
            }
        }
        Ok(val)
    }
}
