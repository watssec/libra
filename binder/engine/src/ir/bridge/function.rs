use std::collections::BTreeSet;

use crate::error::{EngineError, EngineResult, Unsupported};
use crate::ir::adapter;
use crate::ir::bridge::cfg::ControlFlowGraph;
use crate::ir::bridge::intrinsics::filter_intrinsics;
use crate::ir::bridge::shared::{Identifier, SymbolRegistry};
use crate::ir::bridge::typing::{Type, TypeRegistry};

use super::value::RegisterSlot;

/// An adapted representation of an LLVM function parameter
#[derive(Eq, PartialEq)]
pub struct Parameter {
    /// name
    pub name: Option<Identifier>,
    /// declared type
    pub ty: Type,
    /// element annotation
    pub annotated_pointee_type: Option<Type>,
}

/// An adapted representation of an LLVM function
#[derive(Eq, PartialEq)]
pub struct Function {
    /// function name
    pub name: Identifier,
    /// parameter definitions
    pub params: Vec<Parameter>,
    /// has variadic args
    pub variadic: bool,
    /// return type
    pub ret: Option<Type>,
    /// one-definition rule (ODR)
    pub is_weak: bool,
    /// body of the function (in terms of a CFG)
    pub body: Option<ControlFlowGraph>,
}

impl Parameter {
    fn set_or_check_annotated_type(
        current: &mut Option<Type>,
        expected_ty: &Type,
        typing: &TypeRegistry,
        ty: Option<&adapter::typing::Type>,
        tag: &str,
    ) -> EngineResult<()> {
        match ty {
            None => (),
            Some(annotated) => {
                if !matches!(expected_ty, Type::Pointer) {
                    return Err(EngineError::InvalidAssumption(format!(
                        "only pointer parameters can have attribute {}",
                        tag
                    )));
                }
                let converted = typing.convert(annotated)?;
                match current {
                    None => {
                        *current = Some(converted);
                    }
                    Some(existing) => {
                        if existing != &converted {
                            return Err(EngineError::InvalidAssumption(format!(
                                "attribute {} does not match with existing annotation",
                                tag
                            )));
                        }
                    }
                }
            }
        };
        Ok(())
    }

    pub fn convert(
        param: &adapter::function::Parameter,
        expected_ty: &Type,
        typing: &TypeRegistry,
    ) -> EngineResult<Self> {
        let adapter::function::Parameter {
            name: param_name,
            ty: param_ty,
            by_val,
            by_ref,
            pre_allocated,
            struct_ret,
            in_alloca,
            element_type,
        } = param;

        // extract basic type
        let param_ty_new = typing.convert(param_ty)?;
        if &param_ty_new != expected_ty {
            return Err(EngineError::InvalidAssumption(format!(
                "parameter type mismatch: expect {}, actual {}",
                expected_ty, param_ty_new
            )));
        }

        // extract type annotations, if any
        let mut annotated_pointee_type = None;
        Self::set_or_check_annotated_type(
            &mut annotated_pointee_type,
            expected_ty,
            typing,
            by_val.as_ref(),
            "by-val",
        )?;
        Self::set_or_check_annotated_type(
            &mut annotated_pointee_type,
            expected_ty,
            typing,
            by_ref.as_ref(),
            "by-ref",
        )?;
        Self::set_or_check_annotated_type(
            &mut annotated_pointee_type,
            expected_ty,
            typing,
            pre_allocated.as_ref(),
            "pre-allocated",
        )?;
        Self::set_or_check_annotated_type(
            &mut annotated_pointee_type,
            expected_ty,
            typing,
            struct_ret.as_ref(),
            "struct-ret",
        )?;
        Self::set_or_check_annotated_type(
            &mut annotated_pointee_type,
            expected_ty,
            typing,
            in_alloca.as_ref(),
            "in-alloca",
        )?;
        Self::set_or_check_annotated_type(
            &mut annotated_pointee_type,
            expected_ty,
            typing,
            element_type.as_ref(),
            "element-type",
        )?;

        Ok(Parameter {
            name: param_name.as_ref().map(|e| e.into()),
            ty: param_ty_new,
            annotated_pointee_type,
        })
    }
}

impl Function {
    pub fn convert(
        func: &adapter::function::Function,
        typing: &TypeRegistry,
        symbols: &SymbolRegistry,
    ) -> EngineResult<Self> {
        let adapter::function::Function {
            name,
            ty,
            is_defined,
            is_exact,
            is_intrinsic,
            params,
            blocks,
        } = func;

        // convert the name
        let ident: Identifier = name
            .as_ref()
            .ok_or(EngineError::NotSupportedYet(Unsupported::AnonymousFunction))?
            .into();

        // filter intrinsics
        filter_intrinsics(ident.as_ref())?;

        // convert the signature
        let func_ty = typing.convert(ty)?;
        let (param_tys, variadic, ret_ty) = match func_ty {
            Type::Function {
                params,
                variadic,
                ret,
            } => (params, variadic, ret.map(|e| *e)),
            _ => {
                return Err(EngineError::InvalidAssumption(format!(
                    "invalid signature for function: {}",
                    ident
                )));
            }
        };

        // convert parameters
        if params.len() != param_tys.len() {
            return Err(EngineError::InvalidAssumption(format!(
                "parameter count mismatch for function: {}",
                ident
            )));
        }
        let params_new: Vec<_> = params
            .iter()
            .zip(param_tys)
            .map(|(p, t)| Parameter::convert(p, &t, typing))
            .collect::<EngineResult<_>>()?;

        let body = if *is_defined {
            if blocks.is_empty() {
                return Err(EngineError::InvalidAssumption(format!(
                    "a defined function must have at least one basic block: {}",
                    ident
                )));
            }
            if *is_intrinsic {
                return Err(EngineError::InvalidAssumption(format!(
                    "a defined function cannot be an intrinsic: {}",
                    name.as_ref().map_or("<unknown>", |e| e.as_str())
                )));
            }

            // construct the CFG
            Some(ControlFlowGraph::build(
                typing,
                symbols,
                &params_new,
                ret_ty.as_ref(),
                blocks,
            )?)
        } else {
            None
        };

        // done with the construction
        Ok(Self {
            name: ident,
            params: params_new,
            variadic,
            ret: ret_ty,
            is_weak: !*is_exact,
            body,
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
                    "no duplicated function: {}",
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
                return Err(EngineError::NotSupportedYet(Unsupported::WeakFunction));
            }
        }
        Ok(val)
    }

    pub fn collect_variables(&self) -> BTreeSet<RegisterSlot> { 
	let mut result: BTreeSet<RegisterSlot> = BTreeSet::new();
	
	// Ignore parameters for now
	// for param in self.params {
	//     let Some(name) = param.name else { return result };
	//     result.insert(name)
	// }
	
	let Some(body) = &self.body else { return result };
	result.append(&mut body.collect_variables());
	result  
    }
}
