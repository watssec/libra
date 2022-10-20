use llvm_ir::module::{GlobalVariable as LLVMGlobalVariable, ThreadLocalMode};
use llvm_ir::Module as LLVMModule;
use llvm_ir::Name;

use crate::error::{EngineError, Unsupported};
use crate::ir::bridge::constant::Constant;
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
    /// initializer
    pub initializer: Option<Constant>,
}

impl GlobalVariable {
    pub fn convert(
        llvm_module: &LLVMModule,
        llvm_gvar: &LLVMGlobalVariable,
        typing: &TypeRegistry,
    ) -> EngineResult<Self> {
        // filter out unsupported cases
        if !matches!(llvm_gvar.thread_local_mode, ThreadLocalMode::NotThreadLocal) {
            return Err(EngineError::NotSupportedYet(
                Unsupported::ThreadLocalStorage,
            ));
        }
        if llvm_gvar.addr_space != 0 {
            return Err(EngineError::NotSupportedYet(
                Unsupported::PointerAddressSpace,
            ));
        }

        // convert the name
        let name = match &llvm_gvar.name {
            Name::Name(name_str) => name_str.as_ref().into(),
            Name::Number(_) => {
                return Err(EngineError::InvalidAssumption(
                    "no anonymous global variable".into(),
                ));
            }
        };

        // convert the type
        let ref_ty = typing.convert(&llvm_gvar.ty)?;
        let ty = match ref_ty {
            Type::Pointer { pointee } => match pointee {
                None => {
                    return Err(EngineError::InvalidAssumption(
                        "no void content type for global variable".into(),
                    ));
                }
                Some(sub_ty) => sub_ty,
            },
            _ => {
                return Err(EngineError::InvalidAssumption(
                    "all global variables must be pointer type".into(),
                ));
            }
        };

        // convert constant
        let initializer = match &llvm_gvar.initializer {
            None => None,
            Some(llvm_const) => Some(Constant::convert(llvm_module, llvm_const, &ty, typing)?),
        };

        // check that all immutable globals have initializer
        if llvm_gvar.is_constant && initializer.is_none() {
            return Err(EngineError::InvalidAssumption(format!(
                "must have initializer for a constant global: {}",
                llvm_gvar.name
            )));
        }

        // done with the construction
        Ok(Self {
            name,
            ty: *ty,
            initializer,
        })
    }
}
