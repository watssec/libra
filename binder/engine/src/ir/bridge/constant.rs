use std::collections::{BTreeMap, BTreeSet};

use num_bigint::BigUint;
use num_traits::{identities::Zero, Num};

use crate::error::{EngineError, EngineResult, Unsupported};
use crate::ir::adapter;
use crate::ir::bridge::instruction::{BinaryOperator, ComparePredicate, Context, Instruction};
use crate::ir::bridge::shared::{Identifier, SymbolRegistry};
use crate::ir::bridge::typing::{Type, TypeRegistry};

/// A naive translation from an LLVM constant
#[derive(Eq, PartialEq, Clone)]
pub enum Constant {
    /// Integer
    Bitvec { bits: usize, value: BigUint },
    /// Null pointer
    Null,
    /// Array
    Array { sub: Type, elements: Vec<Constant> },
    /// Struct
    Struct {
        name: Option<Identifier>,
        fields: Vec<Constant>,
    },
    /// Global variable
    Variable { name: Identifier },
    /// Function
    Function { name: Identifier },
    /// Undefined bitvec
    UndefBitvec { bits: usize },
    /// Undefined pointer
    UndefPointer,
    /// Expression
    Expr(Box<Expression>),
}

impl Constant {
    fn default_from_type(ty: &Type) -> EngineResult<Self> {
        let value = match ty {
            Type::Bitvec { bits } => Self::Bitvec {
                bits: *bits,
                value: BigUint::zero(),
            },
            Type::Array { element, length } => {
                let elements = (0..*length)
                    .map(|_| Self::default_from_type(element))
                    .collect::<EngineResult<_>>()?;
                Self::Array {
                    sub: element.as_ref().clone(),
                    elements,
                }
            }
            Type::Struct { name, fields } => {
                let defaults = fields
                    .iter()
                    .map(Self::default_from_type)
                    .collect::<EngineResult<_>>()?;
                Self::Struct {
                    name: name.clone(),
                    fields: defaults,
                }
            }
            Type::Function { .. } => {
                return Err(EngineError::InvariantViolation(format!(
                    "trying to create defaults for a function type: {}",
                    ty
                )));
            }
            Type::Pointer => Self::Null,
        };
        Ok(value)
    }

    fn undef_from_type(ty: &Type) -> EngineResult<Self> {
        let value = match ty {
            Type::Bitvec { bits } => Self::UndefBitvec { bits: *bits },
            Type::Array { element, length } => {
                let elements = (0..*length)
                    .map(|_| Self::undef_from_type(element))
                    .collect::<EngineResult<_>>()?;
                Self::Array {
                    sub: element.as_ref().clone(),
                    elements,
                }
            }
            Type::Struct { name, fields } => {
                let defaults = fields
                    .iter()
                    .map(Self::undef_from_type)
                    .collect::<EngineResult<_>>()?;
                Self::Struct {
                    name: name.clone(),
                    fields: defaults,
                }
            }
            Type::Function { .. } => {
                return Err(EngineError::InvariantViolation(format!(
                    "trying to create undef-body for a function type: {}",
                    ty
                )));
            }
            Type::Pointer => Self::UndefPointer,
        };
        Ok(value)
    }

    pub fn convert(
        constant: &adapter::constant::Constant,
        expected_type: &Type,
        typing: &TypeRegistry,
        symbols: &SymbolRegistry,
    ) -> EngineResult<Self> {
        use adapter::constant::Const as AdaptedConst;

        // utility
        let check_type = |ty: &adapter::typing::Type| {
            typing.convert(ty).and_then(|actual_type| {
                if expected_type == &actual_type {
                    Ok(())
                } else {
                    Err(EngineError::InvalidAssumption(format!(
                        "type mismatch: expect {}, found {}",
                        expected_type, actual_type
                    )))
                }
            })
        };

        let adapter::constant::Constant { ty, repr } = constant;

        let result = match repr {
            AdaptedConst::Int { value } => {
                check_type(ty)?;
                match expected_type {
                    Type::Bitvec { bits } => Self::Bitvec {
                        bits: *bits,
                        value: BigUint::from_str_radix(value, 10).map_err(|e| {
                            EngineError::InvariantViolation(format!("const int parse error: {}", e))
                        })?,
                    },
                    _ => {
                        return Err(EngineError::InvalidAssumption(format!(
                            "type mismatch: expect bitvec, found {}",
                            expected_type
                        )));
                    }
                }
            }
            AdaptedConst::Float { .. } => {
                return Err(EngineError::NotSupportedYet(Unsupported::FloatingPoint));
            }
            AdaptedConst::Null => {
                check_type(ty)?;
                if !matches!(expected_type, Type::Pointer) {
                    return Err(EngineError::InvalidAssumption(format!(
                        "type mismatch: expect pointer, found {}",
                        expected_type
                    )));
                }
                Self::Null
            }
            AdaptedConst::None => {
                return Err(EngineError::InvalidAssumption(format!(
                    "unexpected constant none for type: {}",
                    expected_type
                )));
            }
            AdaptedConst::Extension => {
                return Err(EngineError::NotSupportedYet(
                    Unsupported::ArchSpecificExtension,
                ));
            }
            AdaptedConst::Undef => {
                check_type(ty)?;
                Self::undef_from_type(expected_type)?
            }
            AdaptedConst::Default => {
                check_type(ty)?;
                Self::default_from_type(expected_type)?
            }
            AdaptedConst::Array { elements } => {
                check_type(ty)?;
                match expected_type {
                    Type::Array { element, length } => {
                        if elements.len() != *length {
                            return Err(EngineError::InvalidAssumption(format!(
                                "type mismatch: expect {} elements, found {}",
                                length,
                                elements.len()
                            )));
                        }

                        let elements_new = elements
                            .iter()
                            .map(|e| Self::convert(e, element, typing, symbols))
                            .collect::<EngineResult<_>>()?;
                        Self::Array {
                            sub: element.as_ref().clone(),
                            elements: elements_new,
                        }
                    }
                    _ => {
                        return Err(EngineError::InvalidAssumption(format!(
                            "type mismatch: expect array, found {}",
                            expected_type
                        )));
                    }
                }
            }
            AdaptedConst::Vector { .. } => {
                return Err(EngineError::NotSupportedYet(Unsupported::Vectorization));
            }
            AdaptedConst::Struct { elements } => {
                check_type(ty)?;
                match expected_type {
                    Type::Struct { name, fields } => {
                        if elements.len() != fields.len() {
                            return Err(EngineError::InvalidAssumption(format!(
                                "type mismatch: expect {} elements, found {}",
                                fields.len(),
                                elements.len()
                            )));
                        }

                        let elements_new = elements
                            .iter()
                            .zip(fields.iter())
                            .map(|(e, t)| Self::convert(e, t, typing, symbols))
                            .collect::<EngineResult<_>>()?;
                        Self::Struct {
                            name: name.clone(),
                            fields: elements_new,
                        }
                    }
                    _ => {
                        return Err(EngineError::InvalidAssumption(format!(
                            "type mismatch: expect array, found {}",
                            expected_type
                        )));
                    }
                }
            }
            AdaptedConst::Variable { name } => {
                check_type(ty)?;
                if !matches!(expected_type, Type::Pointer) {
                    return Err(EngineError::InvalidAssumption(format!(
                        "type mismatch: expect pointer, found {}",
                        expected_type
                    )));
                }
                match name {
                    None => {
                        return Err(EngineError::InvalidAssumption(
                            "unexpected reference to an anonymous global variable".into(),
                        ));
                    }
                    Some(n) => {
                        let ident = n.into();
                        if !symbols.has_global(&ident) {
                            return Err(EngineError::InvalidAssumption(format!(
                                "unexpected reference to an unknown global variable: {}",
                                ident
                            )));
                        }
                        Self::Variable { name: ident }
                    }
                }
            }
            AdaptedConst::Function { name } => {
                check_type(ty)?;
                if !matches!(expected_type, Type::Pointer) {
                    return Err(EngineError::InvalidAssumption(format!(
                        "type mismatch: expect pointer, found {}",
                        expected_type
                    )));
                }
                match name {
                    None => {
                        return Err(EngineError::InvalidAssumption(
                            "unexpected reference to an anonymous function".into(),
                        ));
                    }
                    Some(n) => {
                        let ident = n.into();
                        if !symbols.has_function(&ident) {
                            return Err(EngineError::InvalidAssumption(format!(
                                "unexpected reference to an unknown function: {}",
                                ident
                            )));
                        }
                        Self::Function { name: ident }
                    }
                }
            }
            AdaptedConst::Alias { .. } => {
                return Err(EngineError::NotSupportedYet(Unsupported::GlobalAlias));
            }
            AdaptedConst::Interface { .. } => {
                return Err(EngineError::NotSupportedYet(Unsupported::InterfaceResolver));
            }
            AdaptedConst::PC => {
                return Err(EngineError::NotSupportedYet(Unsupported::IndirectJump));
            }
            AdaptedConst::Expr { inst } => {
                check_type(ty)?;
                let mut ctxt = Context {
                    typing,
                    symbols,
                    // simulate an environment where there is no function body
                    blocks: BTreeSet::new(),
                    insts: BTreeMap::new(),
                    args: BTreeMap::new(),
                    ret: None,
                };

                // create a dummy instruction
                let fake_inst = adapter::instruction::Instruction {
                    name: None,
                    ty: ty.clone(),
                    index: usize::MAX,
                    repr: inst.as_ref().clone(),
                };
                let inst_parsed = ctxt.parse_instruction(&fake_inst)?;
                let expr_parsed = Expression::from_instruction(inst_parsed)?;
                Self::Expr(Box::new(expr_parsed))
            }
        };
        Ok(result)
    }
}

#[derive(Eq, PartialEq, Clone)]
#[allow(clippy::upper_case_acronyms)]
pub enum Expression {
    // binary
    Binary {
        bits: usize,
        opcode: BinaryOperator,
        lhs: Constant,
        rhs: Constant,
    },
    // comparison
    Compare {
        bits: Option<usize>, // some for bitvec and none for pointer
        predicate: ComparePredicate,
        lhs: Constant,
        rhs: Constant,
    },
    // casts
    CastBitvec {
        bits_from: usize,
        bits_into: usize,
        operand: Constant,
    },
    CastPtrToBitvec {
        bits_into: usize,
        operand: Constant,
    },
    CastBitvecToPtr {
        bits_from: usize,
        operand: Constant,
    },
    CastPtr {
        operand: Constant,
    },
    // GEP
    GEP {
        src_pointee_type: Type,
        dst_pointee_type: Type,
        pointer: Constant,
        offset: Constant,
        indices: Vec<Constant>,
    },
    // choice
    ITE {
        cond: Constant,
        then_value: Constant,
        else_value: Constant,
    },
    // aggregation
    GetValue {
        src_ty: Type,
        dst_ty: Type,
        aggregate: Constant,
        indices: Vec<usize>,
    },
    SetValue {
        src_ty: Type,
        dst_ty: Type,
        aggregate: Constant,
        value: Constant,
        indices: Vec<usize>,
    },
}

impl Expression {
    pub fn from_instruction(inst: Instruction) -> EngineResult<Self> {
        let expr = match inst {
            Instruction::Binary {
                bits,
                opcode,
                lhs,
                rhs,
                result,
            } => {
                assert!(result == usize::MAX.into());
                Self::Binary {
                    bits,
                    opcode,
                    lhs: lhs.expect_constant()?,
                    rhs: rhs.expect_constant()?,
                }
            }
            Instruction::Compare {
                bits,
                predicate,
                lhs,
                rhs,
                result,
            } => {
                assert!(result == usize::MAX.into());
                Self::Compare {
                    bits,
                    predicate,
                    lhs: lhs.expect_constant()?,
                    rhs: rhs.expect_constant()?,
                }
            }
            Instruction::CastBitvec {
                bits_from,
                bits_into,
                operand,
                result,
            } => {
                assert!(result == usize::MAX.into());
                Self::CastBitvec {
                    bits_from,
                    bits_into,
                    operand: operand.expect_constant()?,
                }
            }
            Instruction::CastPtrToBitvec {
                bits_into,
                operand,
                result,
            } => {
                assert!(result == usize::MAX.into());
                Self::CastPtrToBitvec {
                    bits_into,
                    operand: operand.expect_constant()?,
                }
            }
            Instruction::CastBitvecToPtr {
                bits_from,
                operand,
                result,
            } => {
                assert!(result == usize::MAX.into());
                Self::CastBitvecToPtr {
                    bits_from,
                    operand: operand.expect_constant()?,
                }
            }
            Instruction::CastPtr { operand, result } => {
                assert!(result == usize::MAX.into());
                Self::CastPtr {
                    operand: operand.expect_constant()?,
                }
            }
            Instruction::GEP {
                src_pointee_type,
                dst_pointee_type,
                pointer,
                offset,
                indices,
                result,
            } => {
                assert!(result == usize::MAX.into());
                Self::GEP {
                    src_pointee_type,
                    dst_pointee_type,
                    pointer: pointer.expect_constant()?,
                    offset: offset.expect_constant()?,
                    indices: indices
                        .into_iter()
                        .map(|i| i.expect_constant())
                        .collect::<EngineResult<_>>()?,
                }
            }
            Instruction::ITE {
                cond,
                then_value,
                else_value,
                result,
            } => {
                assert!(result == usize::MAX.into());
                Self::ITE {
                    cond: cond.expect_constant()?,
                    then_value: then_value.expect_constant()?,
                    else_value: else_value.expect_constant()?,
                }
            }
            Instruction::GetValue {
                src_ty,
                dst_ty,
                aggregate,
                indices,
                result,
            } => {
                assert!(result == usize::MAX.into());
                Self::GetValue {
                    src_ty,
                    dst_ty,
                    aggregate: aggregate.expect_constant()?,
                    indices,
                }
            }
            Instruction::SetValue {
                src_ty,
                dst_ty,
                aggregate,
                value,
                indices,
                result,
            } => {
                assert!(result == usize::MAX.into());
                Self::SetValue {
                    src_ty,
                    dst_ty,
                    aggregate: aggregate.expect_constant()?,
                    value: value.expect_constant()?,
                    indices,
                }
            }
            // impossible cases
            Instruction::Alloca { .. }
            | Instruction::Load { .. }
            | Instruction::Store { .. }
            | Instruction::CallDirect { .. }
            | Instruction::CallIndirect { .. }
            | Instruction::FreezeBitvec { .. }
            | Instruction::FreezePtr
            | Instruction::Phi { .. } => {
                return Err(EngineError::InvalidAssumption(
                    "unexpected instruction type for const expr".into(),
                ))
            }
        };
        Ok(expr)
    }
}
