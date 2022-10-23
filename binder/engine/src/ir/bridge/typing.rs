use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{Display, Formatter};

use either::Either;
use petgraph::algo::tarjan_scc;
use petgraph::graph::DiGraph;

use crate::error::{EngineError, Unsupported};
use crate::ir::adapter;
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
    Named {
        name: Identifier,
    },
    Function {
        params: Vec<TypeToken>,
        ret: Box<TypeToken>,
    },
    Pointer {
        pointee: Box<TypeToken>,
    },
}

impl TypeToken {
    fn parse(ty: &adapter::typing::Type) -> EngineResult<Self> {
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
                let element_new = Self::parse(element.as_ref())?;
                Self::Array {
                    element: Box::new(element_new),
                    length: *length,
                }
            }
            AdaptedType::Struct { name, fields } => match (name, fields) {
                (None, None) => {
                    return Err(EngineError::InvalidAssumption(
                        "no anonymous and opaque struct".into(),
                    ));
                }
                (Some(ident), None) => Self::Named { name: ident.into() },
                (_, Some(items)) => {
                    let name_new = name.as_ref().map(|ident| ident.into());
                    let fields_new = items
                        .iter()
                        .map(|e| Self::parse(e))
                        .collect::<EngineResult<_>>()?;
                    Self::Struct {
                        name: name_new,
                        fields: fields_new,
                    }
                }
            },
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
                    .map(|e| Self::parse(e))
                    .collect::<EngineResult<_>>()?;
                let ret_new = Self::parse(ret)?;
                Self::Function {
                    params: params_new,
                    ret: Box::new(ret_new),
                }
            }
            AdaptedType::Pointer {
                pointee,
                address_space: _,
            } => {
                let pointee_new = match pointee {
                    None => {
                        return Err(EngineError::NotSupportedYet(Unsupported::OpaquePointerType));
                    }
                    Some(sub_ty) => Self::parse(sub_ty)?,
                };
                Self::Pointer {
                    pointee: Box::new(pointee_new),
                }
            }
            AdaptedType::Vector { .. } => {
                return Err(EngineError::NotSupportedYet(Unsupported::Vectorization));
            }
            AdaptedType::Label | AdaptedType::Token | AdaptedType::Metadata => {
                return Err(EngineError::InvalidAssumption(format!(
                    "no unexpected llvm primitive type: {:?}",
                    ty
                )));
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

    fn deps_recursive(&self, deps: &mut BTreeSet<Identifier>) {
        match self {
            Self::Void | Self::Bitvec { .. } => (),
            Self::Array { element, length: _ } => {
                element.deps_recursive(deps);
            }
            Self::Struct { name: _, fields } => {
                for field in fields {
                    field.deps_recursive(deps);
                }
            }
            Self::Named { name } => {
                deps.insert(name.clone());
            }
            Self::Function { params, ret } => {
                for param in params {
                    param.deps_recursive(deps);
                }
                ret.deps_recursive(deps);
            }
            Self::Pointer { pointee } => {
                pointee.deps_recursive(deps);
            }
        }
    }
}

/// An adapted representation of LLVM typing system
#[derive(Eq, PartialEq, Clone, Debug)]
pub enum Type {
    /// Bit-vector
    Bitvec { bits: usize, mask: u64 },
    /// An array with elements being the same type
    Array { element: Box<Type>, length: usize },
    /// A non-mutually-recursive struct
    StructSimple {
        name: Option<Identifier>,
        fields: Vec<Type>,
    },
    /// A mutually recursive struct with its group
    StructRecursive { name: Identifier },
    /// A function type
    Function {
        params: Vec<Type>,
        ret: Option<Box<Type>>,
    },
    /// A non-void pointer
    Pointer {
        /// A `None` in `pointee` represents a void pointer
        pointee: Option<Box<Type>>,
    },
}

impl Type {
    fn convert_token(
        token: &TypeToken,
        registry: &TypeRegistry,
        current_recursive_group: &BTreeSet<Identifier>,
    ) -> EngineResult<Self> {
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
                let converted = Self::convert_token(element, registry, current_recursive_group)?;
                Self::Array {
                    element: Box::new(converted),
                    length: *length,
                }
            }
            TypeToken::Struct { name, fields } => {
                let converted = fields
                    .iter()
                    .map(|e| Self::convert_token(e, registry, current_recursive_group))
                    .collect::<EngineResult<_>>()?;
                Self::StructSimple {
                    name: name.as_ref().cloned(),
                    fields: converted,
                }
            }
            TypeToken::Named { name } => {
                if current_recursive_group.contains(name) {
                    Self::StructRecursive { name: name.clone() }
                } else {
                    match registry.get_struct(name)? {
                        Either::Left(fields) => Self::StructSimple {
                            name: Some(name.clone()),
                            fields: fields.clone(),
                        },
                        Either::Right(_) => Self::StructRecursive { name: name.clone() },
                    }
                }
            }
            TypeToken::Function { params, ret } => {
                let converted = params
                    .iter()
                    .map(|e| Self::convert_token(e, registry, current_recursive_group))
                    .collect::<EngineResult<_>>()?;

                let new_ret = match ret.as_ref() {
                    TypeToken::Void => None,
                    _ => {
                        let adapted = Self::convert_token(ret, registry, current_recursive_group)?;
                        Some(Box::new(adapted))
                    }
                };
                Self::Function {
                    params: converted,
                    ret: new_ret,
                }
            }
            TypeToken::Pointer { pointee } => match pointee.as_ref() {
                TypeToken::Void => Self::Pointer { pointee: None },
                _ => {
                    let adapted = Self::convert_token(pointee, registry, current_recursive_group)?;
                    Self::Pointer {
                        pointee: Some(Box::new(adapted)),
                    }
                }
            },
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
            Self::StructSimple { name, fields } => {
                let repr: Vec<_> = fields.iter().map(|e| e.to_string()).collect();
                write!(
                    f,
                    "{}{{{}}}",
                    name.as_ref()
                        .map_or_else(|| "".to_string(), |n| n.to_string()),
                    repr.join(",")
                )
            }
            Self::StructRecursive { name } => {
                write!(f, "datatype.{}", name)
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
            Self::Pointer { pointee } => match pointee.as_ref() {
                None => write!(f, "void*"),
                Some(t) => write!(f, "{}*", t),
            },
        }
    }
}

// for simplicity and readability
type RecursiveTypeGroup = BTreeMap<Identifier, Vec<Type>>;

/// A type registry that holds all the user-defined struct types
#[derive(Eq, PartialEq)]
pub struct TypeRegistry {
    struct_simple: BTreeMap<Identifier, Vec<Type>>,
    struct_recursive: BTreeMap<BTreeSet<Identifier>, RecursiveTypeGroup>,
}

impl TypeRegistry {
    fn get_struct(
        &self,
        name: &Identifier,
    ) -> EngineResult<Either<&Vec<Type>, &RecursiveTypeGroup>> {
        // search for simple struct
        if let Some(fields) = self.struct_simple.get(name) {
            return Ok(Either::Left(fields));
        }
        // search for recursive struct
        for (k, v) in self.struct_recursive.iter() {
            if k.contains(name) {
                return Ok(Either::Right(v));
            }
        }
        // the name must be in one of them
        Err(EngineError::InvariantViolation(format!(
            "unprocessed named type: {}",
            name
        )))
    }

    pub fn get_struct_recursive(&self, name: &Identifier) -> EngineResult<&Vec<Type>> {
        for (k, v) in self.struct_recursive.iter() {
            if k.contains(name) {
                return Ok(v.get(name).unwrap());
            }
        }
        Err(EngineError::InvariantViolation(format!(
            "unknown recursive struct: {}",
            name
        )))
    }

    pub fn convert(&self, llvm_type: &adapter::typing::Type) -> EngineResult<Type> {
        let token = TypeToken::parse(llvm_type)?;
        // NOTE: by setting `current_recursive_group`, we force the registry to provide the info
        Type::convert_token(&token, self, &BTreeSet::new())
    }

    pub fn populate(user_defined_structs: &[adapter::typing::Type]) -> EngineResult<Self> {
        use adapter::typing::Type as AdaptedType;

        // collect user-defined structs
        let mut type_graph = DiGraph::new();
        let mut type_ident_to_index = BTreeMap::new();
        let mut type_ident_to_fields = BTreeMap::new();

        for ty in user_defined_structs {
            let (ident, items) = match ty {
                AdaptedType::Struct { name, fields } => {
                    let ident: Identifier = match name {
                        None => {
                            return Err(EngineError::InvalidAssumption(
                                "user-defined struct type cannot be anonymous".into(),
                            ));
                        }
                        Some(n) => n.into(),
                    };
                    let items = match fields {
                        None => {
                            return Err(EngineError::NotSupportedYet(
                                Unsupported::OpaqueStructDefinition,
                            ));
                        }
                        Some(tys) => tys,
                    };
                    (ident, items)
                }
                _ => {
                    return Err(EngineError::InvalidAssumption(
                        "user-defined types must be a struct type".into(),
                    ));
                }
            };

            let index = type_graph.add_node(ident.clone());
            match type_ident_to_index.insert(ident.clone(), index) {
                None => (),
                Some(_) => {
                    return Err(EngineError::InvalidAssumption(format!(
                        "no duplicated definition of struct: {}",
                        ident
                    )));
                }
            }
            assert!(type_ident_to_fields.insert(ident, items).is_none());
        }

        // analyze the definition
        let mut type_defs = BTreeMap::new();
        let mut self_recursive = BTreeSet::new();
        for (src_ident, items) in type_ident_to_fields {
            let src_index = *type_ident_to_index.get(&src_ident).unwrap();

            // convert fields
            let fields: Vec<_> = items
                .iter()
                .map(TypeToken::parse)
                .collect::<EngineResult<_>>()?;

            // check for recursive dependencies
            let mut deps = BTreeSet::new();
            for field in fields.iter() {
                field.deps_recursive(&mut deps);
            }
            // check that all deps are in the allow-list
            for dep_ident in deps.iter() {
                match type_ident_to_index.get(dep_ident) {
                    None => {
                        return Err(EngineError::InvariantViolation(format!(
                            "unknown struct name: {}",
                            dep_ident
                        )));
                    }
                    Some(dep_index) => {
                        type_graph.add_edge(src_index, *dep_index, ());
                    }
                }
            }
            // mark if this struct is self-recursive
            if deps.contains(&src_ident) {
                self_recursive.insert(src_ident.clone());
            }
            // register the definition
            type_defs.insert(src_ident, fields);
        }

        // build the types by SCC in topological order
        let type_sccs = tarjan_scc(&type_graph);

        let mut registry = Self {
            struct_simple: BTreeMap::new(),
            struct_recursive: BTreeMap::new(),
        };
        for scc in type_sccs.into_iter() {
            // collect and sort lexically
            let idents: BTreeSet<_> = scc
                .into_iter()
                .map(|node| type_graph.node_weight(node).unwrap().clone())
                .collect();

            // construct the definition
            let mut group_def = BTreeMap::new();
            for ident in idents.iter() {
                let fields = type_defs.get(ident).unwrap();
                let converted: Vec<_> = fields
                    .iter()
                    .map(|e| Type::convert_token(e, &registry, &idents))
                    .collect::<EngineResult<_>>()?;

                // check whether this is a simple def or a self-recursive def
                if idents.len() == 1 && !self_recursive.contains(ident) {
                    registry.struct_simple.insert(ident.clone(), converted);
                } else {
                    match group_def.insert(ident.clone(), converted) {
                        None => (),
                        Some(_) => {
                            return Err(EngineError::InvariantViolation(format!(
                                "duplicated registration of simple struct: {}",
                                ident
                            )));
                        }
                    }
                }
            }
            if !group_def.is_empty() {
                match registry.struct_recursive.insert(idents, group_def) {
                    None => (),
                    Some(_) => {
                        return Err(EngineError::InvariantViolation(
                            "duplicated registration of recursive struct group".into(),
                        ));
                    }
                }
            }
        }

        // done with the construction
        Ok(registry)
    }
}

impl Display for TypeRegistry {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "[simple structs]")?;
        for (name, fields) in self.struct_simple.iter() {
            let repr: Vec<_> = fields.iter().map(|e| e.to_string()).collect();
            writeln!(f, "{}{{{}}}", name, repr.join(","))?;
        }
        writeln!(f, "[recursive structs]")?;
        for (group, details) in self.struct_recursive.iter() {
            let names: Vec<_> = group.iter().map(|n| n.to_string()).collect();
            writeln!(f, "<{}>", names.join(","))?;
            for (name, fields) in details {
                let repr: Vec<_> = fields.iter().map(|e| e.to_string()).collect();
                writeln!(f, "  {}{{{}}}", name, repr.join(","))?;
            }
        }
        Ok(())
    }
}
