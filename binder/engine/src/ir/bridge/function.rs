use llvm_ir::Module as LLVMModule;
use llvm_ir::{Function as LLVMFunction, Name};

use crate::error::{EngineError, Unsupported};
use crate::ir::bridge::cfg::ControlFlowGraph;
use crate::ir::bridge::shared::Identifier;
use crate::ir::bridge::typing::{Type, TypeRegistry};
use crate::EngineResult;

/// An adapted representation of a *defined* LLVM function
#[derive(Eq, PartialEq)]
pub struct Function {
    /// function name
    pub name: Identifier,
    /// function parameters
    pub params: Vec<(Identifier, Type)>,
    /// function return type
    pub ret: Type,
    /// function body
    pub cfg: ControlFlowGraph,
}

impl Function {
    pub fn convert(
        llvm_module: &LLVMModule,
        llvm_func: &LLVMFunction,
        typing: &TypeRegistry,
    ) -> EngineResult<Self> {
        // filter out unsupported cases
        if llvm_func.is_var_arg {
            return Err(EngineError::NotSupportedYet(Unsupported::VariadicArguments));
        }
        if llvm_func.garbage_collector_name.is_some() {
            return Err(EngineError::LLVMLoadingError(format!(
                "unexpected garbage collector in function: {}",
                llvm_func.name
            )));
        }
        if llvm_func.personality_function.is_some() {
            return Err(EngineError::LLVMLoadingError(format!(
                "unexpected personality in function: {}",
                llvm_func.name
            )));
        }

        // parse the parameters
        let mut params = vec![];
        for param in llvm_func.parameters.iter() {
            // convert the name
            let name = match &param.name {
                Name::Name(name_str) => name_str.as_ref().into(),
                Name::Number(_) => {
                    return Err(EngineError::InvalidAssumption(
                        "no anonymous function parameter".into(),
                    ));
                }
            };
            let ty = typing.convert(&param.ty)?;
            // TODO: handle the attributes, some attributes seem to be useful
            params.push((name, ty));
        }

        // parse the return type
        let ret = typing.convert(&llvm_func.return_type)?;
        // TODO: handle the return attributes

        // build the cfg
        let cfg = ControlFlowGraph::build()?;

        // TODO: handle the function attributes
        // done with the construction
        Ok(Self {
            name: llvm_func.name.as_str().into(),
            params,
            ret,
            cfg,
        })
    }
}
