use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};

use crate::error::{EngineError, Unsupported};
use crate::ir::adapter;
use crate::ir::adapter::typing::UserDefinedStruct;
use crate::ir::bridge::shared::Identifier;
use crate::EngineResult;

/// A naive translation from an LLVM type
#[derive(Eq, PartialEq)]
enum TypeToken {
    Void,
    Bitvec {
        width: usize,
        mask: u64,
    },
    // TODO: floating point types
    Array {
        element: Box<TypeToken>,
        length: usize,
    },
    Struct {
        name: Option<Identifier>,
        fields: Vec<TypeToken>,
    },
    Function {
        params: Vec<TypeToken>,
        ret: Box<TypeToken>,
    },
    Pointer, // opaque pointer scheme has no pointee type
}

impl TypeToken {
    fn parse(
        ty: &adapter::typing::Type,
        user_defined_structs: &BTreeMap<Identifier, Vec<adapter::typing::Type>>,
    ) -> EngineResult<Self> {
        use adapter::typing::Type as AdaptedType;

        let converted = match ty {
            AdaptedType::Void => Self::Void,
            AdaptedType::Int { width, mask } => Self::Bitvec {
                width: *width,
                mask: *mask,
            },
            AdaptedType::Float { .. } => {
                return Err(EngineError::NotSupportedYet(Unsupported::FloatingPoint));
            }
            AdaptedType::Array { element, length } => {
                let element_new = Self::parse(element.as_ref(), user_defined_structs)?;
                Self::Array {
                    element: Box::new(element_new),
                    length: *length,
                }
            }
            AdaptedType::Struct { name, fields } => {
                let field_tys = match fields {
                    None => {
                        return Err(EngineError::InvalidAssumption(
                            "no opaque struct under opaque pointer scheme".into(),
                        ));
                    }
                    Some(tys) => tys,
                };
                let name_new = name.as_ref().map(|ident| ident.into());

                // sanity check
                match name_new.as_ref() {
                    None => (),
                    Some(ident) => match user_defined_structs.get(ident) {
                        None => {
                            return Err(EngineError::InvalidAssumption(format!(
                                "reference to undefined named struct: {}",
                                ident
                            )));
                        }
                        Some(defined_tys) => {
                            if defined_tys != field_tys {
                                return Err(EngineError::InvalidAssumption(format!(
                                    "conflicting definition of named struct: {}",
                                    ident
                                )));
                            }
                        }
                    },
                }

                // construct the new type
                let fields_new = field_tys
                    .iter()
                    .map(|e| Self::parse(e, user_defined_structs))
                    .collect::<EngineResult<_>>()?;
                Self::Struct {
                    name: name_new,
                    fields: fields_new,
                }
            }
            AdaptedType::Function {
                params,
                variadic,
                ret,
            } => {
                if *variadic {
                    return Err(EngineError::NotSupportedYet(Unsupported::VariadicArguments));
                }
                let params_new = params
                    .iter()
                    .map(|e| Self::parse(e, user_defined_structs))
                    .collect::<EngineResult<_>>()?;
                let ret_new = Self::parse(ret, user_defined_structs)?;
                Self::Function {
                    params: params_new,
                    ret: Box::new(ret_new),
                }
            }
            AdaptedType::Pointer {
                pointee,
                address_space: _,
            } => {
                match pointee {
                    None => (),
                    Some(sub_ty) => {
                        return Err(EngineError::InvalidAssumption(format!(
                            "all pointer type should be opaque, received a pointee `{}` instead",
                            serde_json::to_string(sub_ty).unwrap_or_else(|e| e.to_string()),
                        )));
                    }
                };
                Self::Pointer
            }
            AdaptedType::Vector { .. } => {
                return Err(EngineError::NotSupportedYet(Unsupported::Vectorization));
            }
            AdaptedType::Label => {
                return Err(EngineError::InvalidAssumption(
                    "no unexpected llvm primitive type: label".into(),
                ));
            }
            AdaptedType::Token => {
                return Err(EngineError::InvalidAssumption(
                    "no unexpected llvm primitive type: token".into(),
                ));
            }
            AdaptedType::Metadata => {
                return Err(EngineError::InvalidAssumption(
                    "no unexpected llvm primitive type: metadata".into(),
                ));
            }
            AdaptedType::Other { name } => {
                return Err(EngineError::InvalidAssumption(format!(
                    "no unexpected llvm complex type: {}",
                    name
                )));
            }
        };
        Ok(converted)
    }
}

/// An adapted representation of LLVM typing system
#[derive(Eq, PartialEq, Clone, Debug)]
pub enum Type {
    /// Bit-vector
    Bitvec { bits: usize, mask: u64 },
    /// An array with elements being the same type
    Array { element: Box<Type>, length: usize },
    /// A struct type, named or anonymous
    Struct {
        name: Option<Identifier>,
        fields: Vec<Type>,
    },
    /// A function type
    Function {
        params: Vec<Type>,
        ret: Option<Box<Type>>,
    },
    /// An opaque pointer (i.e., any pointee type is valid)
    Pointer,
}

impl Type {
    fn convert_token(token: &TypeToken) -> EngineResult<Self> {
        let ty = match token {
            TypeToken::Void => {
                return Err(EngineError::InvariantViolation(
                    "unexpected void type".into(),
                ));
            }
            TypeToken::Bitvec { width, mask } => Self::Bitvec {
                bits: *width,
                mask: *mask,
            },
            TypeToken::Array { element, length } => {
                let converted = Self::convert_token(element)?;
                Self::Array {
                    element: Box::new(converted),
                    length: *length,
                }
            }
            TypeToken::Struct { name, fields } => {
                let converted = fields
                    .iter()
                    .map(|e| Self::convert_token(e))
                    .collect::<EngineResult<_>>()?;
                Self::Struct {
                    name: name.as_ref().cloned(),
                    fields: converted,
                }
            }
            TypeToken::Function { params, ret } => {
                let converted = params
                    .iter()
                    .map(|e| Self::convert_token(e))
                    .collect::<EngineResult<_>>()?;

                let new_ret = match ret.as_ref() {
                    TypeToken::Void => None,
                    _ => {
                        let adapted = Self::convert_token(ret)?;
                        Some(Box::new(adapted))
                    }
                };
                Self::Function {
                    params: converted,
                    ret: new_ret,
                }
            }
            TypeToken::Pointer => Self::Pointer,
        };
        Ok(ty)
    }
}

impl Display for Type {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Bitvec { bits, mask } => {
                write!(f, "int{}/{}", bits, mask)
            }
            Self::Array { element, length } => {
                write!(f, "{}[{}]", element, length)
            }
            Self::Struct { name, fields } => {
                let repr: Vec<_> = fields.iter().map(|e| e.to_string()).collect();
                write!(
                    f,
                    "{}{{{}}}",
                    name.as_ref()
                        .map_or_else(|| "<anonymous>".to_string(), |n| n.to_string()),
                    repr.join(",")
                )
            }
            Self::Function { params, ret } => {
                let repr: Vec<_> = params.iter().map(|e| e.to_string()).collect();
                write!(
                    f,
                    "({})->{}",
                    repr.join(","),
                    ret.as_ref()
                        .map_or_else(|| "void".to_string(), |t| { t.to_string() })
                )
            }
            Self::Pointer => write!(f, "ptr"),
        }
    }
}

/// A type registry that holds all the user-defined struct types
#[derive(Eq, PartialEq)]
pub struct TypeRegistry {
    user_defined_structs: BTreeMap<Identifier, Vec<adapter::typing::Type>>,
}

impl TypeRegistry {
    pub fn convert(&self, ty: &adapter::typing::Type) -> EngineResult<Type> {
        let token = TypeToken::parse(ty, &self.user_defined_structs)?;
        Type::convert_token(&token)
    }

    pub fn populate(user_defined_structs: &[UserDefinedStruct]) -> EngineResult<Self> {
        // collect user-defined structs
        let mut type_ident_to_fields = BTreeMap::new();

        for def in user_defined_structs {
            let UserDefinedStruct { name, fields } = def;
            let ident: Identifier = name
                .as_ref()
                .ok_or_else(|| {
                    EngineError::InvalidAssumption(
                        "user-defined struct type cannot be anonymous".into(),
                    )
                })?
                .into();
            let items = fields
                .as_ref()
                .ok_or_else(|| {
                    EngineError::InvalidAssumption(
                        "user-defined struct type cannot be opaque".into(),
                    )
                })?
                .clone();

            match type_ident_to_fields.insert(ident, items) {
                None => (),
                Some(_) => {
                    return Err(EngineError::InvalidAssumption(format!(
                        "no duplicated definition of struct: {}",
                        name.as_ref().unwrap()
                    )));
                }
            }
        }

        // analyze their definitions
        let mut type_defs = BTreeMap::new();
        for (src_ident, items) in type_ident_to_fields.iter() {
            // convert fields
            let fields: Vec<_> = items
                .iter()
                .map(|e| TypeToken::parse(e, &type_ident_to_fields))
                .collect::<EngineResult<_>>()?;

            // register the definition
            assert!(type_defs.insert(src_ident, fields).is_none());
        }

        // done
        Ok(Self {
            user_defined_structs: type_ident_to_fields,
        })
    }
}
