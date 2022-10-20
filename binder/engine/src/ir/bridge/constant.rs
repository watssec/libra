use llvm_ir::types::Typed;
use llvm_ir::{Constant as LLVMConstant, Name};
use llvm_ir::{IntPredicate, Module as LLVMModule};

use crate::error::{EngineError, Unsupported};
use crate::ir::bridge::shared::Identifier;
use crate::ir::bridge::typing::{Type, TypeRegistry};
use crate::EngineResult;

/// An adapted representation of an LLVM constant
#[derive(Eq, PartialEq, Clone, Debug)]
pub enum Constant {
    // base types
    Bitvec {
        bits: u32,
        value: u128,
    },
    // TODO: floating point types
    Array {
        elements: Vec<Constant>,
    },
    Struct {
        fields: Vec<Constant>,
    },
    Null,
    // expressions
    // TODO: global reference should link to global variable
    GlobalReference(Identifier),
    // integer binary ops
    Add(Box<Constant>, Box<Constant>),
    Sub(Box<Constant>, Box<Constant>),
    Mul(Box<Constant>, Box<Constant>),
    UDiv(Box<Constant>, Box<Constant>),
    SDiv(Box<Constant>, Box<Constant>),
    URem(Box<Constant>, Box<Constant>),
    SRem(Box<Constant>, Box<Constant>),
    // integer comparison ops
    IEQ(Box<Constant>, Box<Constant>),
    INE(Box<Constant>, Box<Constant>),
    IUGT(Box<Constant>, Box<Constant>),
    IUGE(Box<Constant>, Box<Constant>),
    IULT(Box<Constant>, Box<Constant>),
    IULE(Box<Constant>, Box<Constant>),
    ISGT(Box<Constant>, Box<Constant>),
    ISGE(Box<Constant>, Box<Constant>),
    ISLT(Box<Constant>, Box<Constant>),
    ISLE(Box<Constant>, Box<Constant>),
    // bitvec bitwise ops
    And(Box<Constant>, Box<Constant>),
    Or(Box<Constant>, Box<Constant>),
    Xor(Box<Constant>, Box<Constant>),
    Shl(Box<Constant>, Box<Constant>),
    LShr(Box<Constant>, Box<Constant>),
    AShr(Box<Constant>, Box<Constant>),
    // bitvec cast ops
    Trunc(Box<Constant>),
    ZExt(Box<Constant>),
    SExt(Box<Constant>),
    PtrToInt(Box<Constant>),
    IntToPtr(Box<Constant>),
    BitCast(Box<Constant>),
    // aggregate ops
    GetValue {
        aggregate: Box<Constant>,
        indices: Vec<usize>,
    },
    SetValue {
        aggregate: Box<Constant>,
        element: Box<Constant>,
        indices: Vec<usize>,
    },
    // memory pointer
    GetElementPtr {
        target: Box<Constant>,
        indices: Vec<Constant>,
    },
    // if-then-else
    Select {
        condition: Box<Constant>,
        then_val: Box<Constant>,
        else_val: Box<Constant>,
    },
}

impl Constant {
    fn default_from_type(ty: &Type, type_registry: &TypeRegistry) -> EngineResult<Self> {
        let result = match ty {
            Type::Bitvec { bits } => Self::Bitvec {
                bits: *bits,
                value: 0,
            },
            Type::Array { element, length } => {
                let default = Self::default_from_type(element, type_registry)?;
                Self::Array {
                    elements: vec![default; *length],
                }
            }
            Type::StructSimple { name: _, fields } => {
                let defaults = fields
                    .iter()
                    .map(|t| Self::default_from_type(t, type_registry))
                    .collect::<EngineResult<_>>()?;
                Self::Struct { fields: defaults }
            }
            Type::StructRecursive { name } => {
                let fields = type_registry.get_struct_recursive(name)?;
                let defaults = fields
                    .iter()
                    .map(|t| Self::default_from_type(t, type_registry))
                    .collect::<EngineResult<_>>()?;
                Self::Struct { fields: defaults }
            }
            Type::Function { .. } => {
                return Err(EngineError::InvariantViolation(format!(
                    "trying to create defaults for a function type: {}",
                    ty
                )));
            }
            Type::Pointer { .. } => Self::Null,
        };
        Ok(result)
    }

    fn parse_const_expr(
        llvm_module: &LLVMModule,
        llvm_const: &LLVMConstant,
        expected_type: &Type,
        type_registry: &TypeRegistry,
    ) -> EngineResult<Self> {
        let mk_type_mismatch = || {
            EngineError::LLVMLoadingError(format!(
                "type mismatch: expect {}, found {}",
                expected_type, llvm_const
            ))
        };

        macro_rules! mk_bin_arithmetic_op {
            ($op:ident, $details:ident) => {{
                let lhs = Self::convert(
                    llvm_module,
                    &$details.operand0,
                    expected_type,
                    type_registry,
                )?;
                let rhs = Self::convert(
                    llvm_module,
                    &$details.operand1,
                    expected_type,
                    type_registry,
                )?;
                Self::$op(Box::new(lhs), Box::new(rhs))
            }};
        }
        macro_rules! mk_bin_comparison_op {
            ($op:ident, $details:ident, $expected_type:ident) => {{
                let lhs = Self::convert(
                    llvm_module,
                    &$details.operand0,
                    &$expected_type,
                    type_registry,
                )?;
                let rhs = Self::convert(
                    llvm_module,
                    &$details.operand1,
                    &$expected_type,
                    type_registry,
                )?;
                Self::$op(Box::new(lhs), Box::new(rhs))
            }};
        }
        macro_rules! mk_bin_bitwise_op {
            ($op:ident, $details:ident) => {{
                let lhs = Self::convert(
                    llvm_module,
                    &$details.operand0,
                    expected_type,
                    type_registry,
                )?;
                let rhs = Self::convert(
                    llvm_module,
                    &$details.operand1,
                    expected_type,
                    type_registry,
                )?;
                Self::$op(Box::new(lhs), Box::new(rhs))
            }};
        }
        macro_rules! mk_unary_cast_op {
            ($op:ident, $details:ident) => {{
                let opv_type = type_registry
                    .convert($details.operand.get_type(&llvm_module.types).as_ref())?;
                // TODO: check cast compatibility
                let opv = Self::convert(llvm_module, &$details.operand, &opv_type, type_registry)?;
                Self::$op(Box::new(opv))
            }};
        }

        // type check
        let const_type = type_registry.convert(llvm_const.get_type(&llvm_module.types).as_ref())?;
        if expected_type != &const_type {
            return Err(mk_type_mismatch());
        }

        // case on the expression
        let result = match llvm_const {
            // cases handled before
            LLVMConstant::Int { .. }
            | LLVMConstant::Float(..)
            | LLVMConstant::Null(..)
            | LLVMConstant::AggregateZero(..)
            | LLVMConstant::Struct { .. }
            | LLVMConstant::Array { .. }
            | LLVMConstant::Vector(..)
            | LLVMConstant::Undef(..)
            | LLVMConstant::Poison(..)
            | LLVMConstant::BlockAddress
            | LLVMConstant::TokenNone => {
                unreachable!("handled before");
            }
            // reference to global variables
            LLVMConstant::GlobalReference { name, ty: _ } => {
                // TODO: pass in the the global variables to check
                // TODO: this assumed that there are no global aliases
                let is_valid = llvm_module.global_vars.iter().any(|v| name == &v.name);
                if !is_valid {
                    return Err(EngineError::LLVMLoadingError(format!(
                        "constant referencing to an non-existent global: {}",
                        llvm_const
                    )));
                }
                let ident = match name {
                    Name::Name(name_str) => name_str.as_str().into(),
                    Name::Number(..) => {
                        return Err(EngineError::InvalidAssumption(
                            "no anonymous global variable".into(),
                        ));
                    }
                };
                Self::GlobalReference(ident)
            }

            // int arithmetics
            LLVMConstant::Add(details) => mk_bin_arithmetic_op!(Add, details),
            LLVMConstant::Sub(details) => mk_bin_arithmetic_op!(Sub, details),
            LLVMConstant::Mul(details) => mk_bin_arithmetic_op!(Mul, details),
            LLVMConstant::UDiv(details) => mk_bin_arithmetic_op!(UDiv, details),
            LLVMConstant::SDiv(details) => mk_bin_arithmetic_op!(SDiv, details),
            LLVMConstant::URem(details) => mk_bin_arithmetic_op!(URem, details),
            LLVMConstant::SRem(details) => mk_bin_arithmetic_op!(SRem, details),
            // int comparison
            LLVMConstant::ICmp(details) => {
                let operand_type = type_registry
                    .convert(details.operand0.get_type(&llvm_module.types).as_ref())?;
                match &details.predicate {
                    IntPredicate::EQ => mk_bin_comparison_op!(IEQ, details, operand_type),
                    IntPredicate::NE => mk_bin_comparison_op!(INE, details, operand_type),
                    IntPredicate::UGT => mk_bin_comparison_op!(IUGT, details, operand_type),
                    IntPredicate::UGE => mk_bin_comparison_op!(IUGE, details, operand_type),
                    IntPredicate::ULT => mk_bin_comparison_op!(IULT, details, operand_type),
                    IntPredicate::ULE => mk_bin_comparison_op!(IULE, details, operand_type),
                    IntPredicate::SGT => mk_bin_comparison_op!(ISGT, details, operand_type),
                    IntPredicate::SGE => mk_bin_comparison_op!(ISGE, details, operand_type),
                    IntPredicate::SLT => mk_bin_comparison_op!(ISLT, details, operand_type),
                    IntPredicate::SLE => mk_bin_comparison_op!(ISLE, details, operand_type),
                }
            }
            // bitwise operations
            LLVMConstant::And(details) => mk_bin_bitwise_op!(And, details),
            LLVMConstant::Or(details) => mk_bin_bitwise_op!(Or, details),
            LLVMConstant::Xor(details) => mk_bin_bitwise_op!(Xor, details),
            LLVMConstant::Shl(details) => mk_bin_bitwise_op!(Shl, details),
            LLVMConstant::LShr(details) => mk_bin_bitwise_op!(LShr, details),
            LLVMConstant::AShr(details) => mk_bin_bitwise_op!(AShr, details),
            // cast operations
            LLVMConstant::Trunc(details) => mk_unary_cast_op!(Trunc, details),
            LLVMConstant::ZExt(details) => mk_unary_cast_op!(ZExt, details),
            LLVMConstant::SExt(details) => mk_unary_cast_op!(SExt, details),
            LLVMConstant::PtrToInt(details) => mk_unary_cast_op!(PtrToInt, details),
            LLVMConstant::IntToPtr(details) => mk_unary_cast_op!(IntToPtr, details),
            LLVMConstant::BitCast(details) => mk_unary_cast_op!(BitCast, details),
            LLVMConstant::AddrSpaceCast(..) => {
                return Err(EngineError::NotSupportedYet(
                    Unsupported::PointerAddressSpace,
                ));
            }
            // floating-point operations
            LLVMConstant::FAdd(..)
            | LLVMConstant::FSub(..)
            | LLVMConstant::FMul(..)
            | LLVMConstant::FDiv(..)
            | LLVMConstant::FRem(..)
            | LLVMConstant::FCmp(..)
            | LLVMConstant::FPTrunc(..)
            | LLVMConstant::FPExt(..)
            | LLVMConstant::FPToUI(..)
            | LLVMConstant::FPToSI(..)
            | LLVMConstant::UIToFP(..)
            | LLVMConstant::SIToFP(..) => {
                return Err(EngineError::NotSupportedYet(Unsupported::FloatingPoint));
            }
            // vector operations
            LLVMConstant::ExtractElement(..)
            | LLVMConstant::InsertElement(..)
            | LLVMConstant::ShuffleVector(..) => {
                return Err(EngineError::NotSupportedYet(Unsupported::Vectorization));
            }
            // aggregates
            LLVMConstant::ExtractValue(details) => {
                let aggregate_type = type_registry
                    .convert(details.aggregate.get_type(&llvm_module.types).as_ref())?;
                let converted_aggregate = Self::convert(
                    llvm_module,
                    &details.aggregate,
                    &aggregate_type,
                    type_registry,
                )?;
                // TODO: walk down the path for type conformance check
                Self::GetValue {
                    aggregate: Box::new(converted_aggregate),
                    indices: details.indices.iter().map(|i| *i as usize).collect(),
                }
            }
            LLVMConstant::InsertValue(details) => {
                let converted_aggregate = Self::convert(
                    llvm_module,
                    &details.aggregate,
                    expected_type,
                    type_registry,
                )?;
                let element_type =
                    type_registry.convert(details.element.get_type(&llvm_module.types).as_ref())?;
                let converted_element =
                    Self::convert(llvm_module, &details.element, &element_type, type_registry)?;
                // TODO: walk down the path for type conformance check
                Self::SetValue {
                    aggregate: Box::new(converted_aggregate),
                    element: Box::new(converted_element),
                    indices: details.indices.iter().map(|i| *i as usize).collect(),
                }
            }
            // memory operations
            LLVMConstant::GetElementPtr(details) => {
                if !details.in_bounds {
                    return Err(EngineError::NotSupportedYet(
                        Unsupported::OutOfBoundConstantGEP,
                    ));
                }

                let target_type =
                    type_registry.convert(details.address.get_type(&llvm_module.types).as_ref())?;
                let converted_target =
                    Self::convert(llvm_module, &details.address, &target_type, type_registry)?;

                let mut converted_indices = vec![];
                for ind in details.indices.iter() {
                    let ind_type =
                        type_registry.convert(ind.get_type(&llvm_module.types).as_ref())?;
                    match ind_type {
                        Type::Bitvec { .. } => (),
                        _ => {
                            return Err(EngineError::InvalidAssumption(
                                "Constant GEP indices are not integers".into(),
                            ));
                        }
                    }
                    // TODO: walk down the indices??
                    let converted_ind = Self::convert(llvm_module, ind, &ind_type, type_registry)?;
                    converted_indices.push(converted_ind);
                }
                Self::GetElementPtr {
                    target: Box::new(converted_target),
                    indices: converted_indices,
                }
            }
            // if-then-else
            LLVMConstant::Select(details) => {
                let cond_type = type_registry
                    .convert(details.condition.get_type(&llvm_module.types).as_ref())?;
                let converted_cond =
                    Self::convert(llvm_module, &details.condition, &cond_type, type_registry)?;
                let then_val = Self::convert(
                    llvm_module,
                    &details.true_value,
                    expected_type,
                    type_registry,
                )?;
                let else_val = Self::convert(
                    llvm_module,
                    &details.false_value,
                    expected_type,
                    type_registry,
                )?;
                Self::Select {
                    condition: Box::new(converted_cond),
                    then_val: Box::new(then_val),
                    else_val: Box::new(else_val),
                }
            }
        };
        Ok(result)
    }

    pub fn convert(
        llvm_module: &LLVMModule,
        llvm_const: &LLVMConstant,
        expected_type: &Type,
        type_registry: &TypeRegistry,
    ) -> EngineResult<Self> {
        let mk_type_mismatch = || {
            EngineError::LLVMLoadingError(format!(
                "type mismatch: expect {}, found {}",
                expected_type, llvm_const
            ))
        };

        // conversion of the basic items
        let result = match llvm_const {
            LLVMConstant::Int { bits, value } => match expected_type {
                Type::Bitvec {
                    bits: expected_bits,
                } => {
                    if bits != expected_bits {
                        return Err(mk_type_mismatch());
                    }
                    Self::Bitvec {
                        bits: *bits,
                        value: *value as u128,
                    }
                }
                _ => {
                    return Err(mk_type_mismatch());
                }
            },
            LLVMConstant::Float(_) => {
                return Err(EngineError::NotSupportedYet(Unsupported::FloatingPoint));
            }
            LLVMConstant::AggregateZero(target) => {
                let obtained = type_registry.convert(target.as_ref())?;
                if expected_type != &obtained {
                    return Err(mk_type_mismatch());
                }
                Self::default_from_type(expected_type, type_registry)?
            }
            LLVMConstant::Array {
                element_type,
                elements,
            } => {
                let obtained = type_registry.convert(element_type.as_ref())?;
                match expected_type {
                    Type::Array {
                        element: expected_element,
                        length: expected_length,
                    } => {
                        if expected_element.as_ref() != &obtained {
                            return Err(mk_type_mismatch());
                        }
                        if elements.len() != *expected_length {
                            return Err(mk_type_mismatch());
                        }
                        let converted = elements
                            .iter()
                            .map(|e| Self::convert(llvm_module, e, expected_element, type_registry))
                            .collect::<EngineResult<_>>()?;
                        Self::Array {
                            elements: converted,
                        }
                    }
                    _ => {
                        return Err(mk_type_mismatch());
                    }
                }
            }
            LLVMConstant::Struct {
                name: _,
                values,
                is_packed: _,
            } => {
                let expected_fields = match expected_type {
                    Type::StructSimple {
                        name: _,
                        fields: expected_fields,
                    } => expected_fields,
                    Type::StructRecursive {
                        name: expected_name,
                    } => type_registry.get_struct_recursive(expected_name)?,
                    _ => {
                        return Err(mk_type_mismatch());
                    }
                };

                // construct the constant with expected fields
                if values.len() != expected_fields.len() {
                    return Err(mk_type_mismatch());
                }
                let converted = values
                    .iter()
                    .zip(expected_fields.iter())
                    .map(|(v, t)| Self::convert(llvm_module, v, t, type_registry))
                    .collect::<EngineResult<_>>()?;
                Self::Struct { fields: converted }
            }
            LLVMConstant::Vector(..) => {
                return Err(EngineError::NotSupportedYet(Unsupported::Vectorization));
            }
            LLVMConstant::Null(target) => {
                let obtained = type_registry.convert(target.as_ref())?;
                if expected_type != &obtained {
                    return Err(mk_type_mismatch());
                }
                match expected_type {
                    Type::Pointer { .. } => Self::Null,
                    _ => {
                        return Err(EngineError::InvalidAssumption(format!(
                            "null pointer assigned to a non-pointer type: {}",
                            llvm_const
                        )));
                    }
                }
            }
            // invalid cases
            LLVMConstant::Undef(..)
            | LLVMConstant::Poison(..)
            | LLVMConstant::BlockAddress
            | LLVMConstant::TokenNone => {
                return Err(EngineError::InvalidAssumption(format!(
                    "no unexpected constant types: {}",
                    llvm_const
                )));
            }
            // the rest falls in the line of constant expressions
            _ => Self::parse_const_expr(llvm_module, llvm_const, expected_type, type_registry)?,
        };
        Ok(result)
    }
}
