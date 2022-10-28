use crate::error::{EngineError, Unsupported};
use crate::ir::adapter;
use crate::ir::bridge::shared::{Identifier, SymbolRegistry};
use crate::ir::bridge::typing::{Type, TypeRegistry};
use crate::EngineResult;

/// A naive translation from an LLVM constant
#[derive(Eq, PartialEq, Clone, Debug)]
pub enum Constant {
    /// Integer
    Bitvec { bits: usize, value: u64 },
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
}

impl Constant {
    fn default_from_type(ty: &Type) -> EngineResult<Self> {
        let value = match ty {
            Type::Bitvec { bits } => Self::Bitvec {
                bits: *bits,
                value: 0,
            },
            Type::Array { element, length } => {
                let default = Self::default_from_type(element)?;
                Self::Array {
                    sub: element.as_ref().clone(),
                    elements: vec![default; *length],
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

    pub fn convert(
        constant: &adapter::constant::Constant,
        expected_type: &Type,
        typing: &TypeRegistry,
        symbols: &SymbolRegistry,
    ) -> EngineResult<Self> {
        use adapter::constant::Constant as AdaptedConstant;

        // utility
        let check_type = |ty: &adapter::typing::Type| {
            typing.convert(ty).and_then(|actual_type| {
                if expected_type == &actual_type {
                    Ok(())
                } else {
                    Err(EngineError::LLVMLoadingError(format!(
                        "type mismatch: expect {}, found {}",
                        expected_type, actual_type
                    )))
                }
            })
        };

        let result = match constant {
            AdaptedConstant::Int { ty, value } => {
                check_type(ty)?;
                match expected_type {
                    Type::Bitvec { bits } => Self::Bitvec {
                        bits: *bits,
                        value: *value,
                    },
                    _ => {
                        return Err(EngineError::InvalidAssumption(format!(
                            "type mismatch: expect bitvec, found {}",
                            expected_type
                        )));
                    }
                }
            }
            AdaptedConstant::Float { .. } => {
                return Err(EngineError::NotSupportedYet(Unsupported::FloatingPoint));
            }
            AdaptedConstant::Null { ty } => {
                check_type(ty)?;
                if !matches!(expected_type, Type::Pointer) {
                    return Err(EngineError::InvalidAssumption(format!(
                        "type mismatch: expect pointer, found {}",
                        expected_type
                    )));
                }
                Self::Null
            }
            AdaptedConstant::None { .. } => {
                return Err(EngineError::InvalidAssumption(format!(
                    "unexpected constant none for type: {}",
                    expected_type
                )));
            }
            AdaptedConstant::Undef { .. } => {
                return Err(EngineError::InvalidAssumption(format!(
                    "unexpected constant undef for type: {}",
                    expected_type
                )));
            }
            AdaptedConstant::Default { ty } => {
                check_type(ty)?;
                Self::default_from_type(expected_type)?
            }
            AdaptedConstant::Array { ty, elements } => {
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
            AdaptedConstant::Vector { .. } => {
                return Err(EngineError::NotSupportedYet(Unsupported::Vectorization));
            }
            AdaptedConstant::Struct { ty, elements } => {
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
            AdaptedConstant::Variable { ty, name } => {
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
                            "no reference to an anonymous global variable".into(),
                        ));
                    }
                    Some(n) => {
                        let ident = n.into();
                        if !symbols.has_global(&ident) {
                            return Err(EngineError::InvalidAssumption(format!(
                                "reference to an unknown global variable: {}",
                                ident
                            )));
                        }
                        Self::Variable { name: ident }
                    }
                }
            }
            AdaptedConstant::Function { ty, name } => {
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
                            "no reference to an anonymous function".into(),
                        ));
                    }
                    Some(n) => {
                        let ident = n.into();
                        if !symbols.has_function(&ident) {
                            return Err(EngineError::InvalidAssumption(format!(
                                "reference to an unknown function: {}",
                                ident
                            )));
                        }
                        Self::Function { name: ident }
                    }
                }
            }
            AdaptedConstant::Alias { .. } => {
                return Err(EngineError::NotSupportedYet(Unsupported::GlobalAlias));
            }
            AdaptedConstant::Interface { .. } => {
                return Err(EngineError::NotSupportedYet(Unsupported::InterfaceResolver));
            }
        };
        Ok(result)
    }
}
