use serde::{Deserialize, Serialize};

/// A representation of an LLVM type
#[derive(Serialize, Deserialize, Eq, PartialEq, Clone)]
pub enum Type {
    /// Void type
    Void,
    /// Integer type represented as bitvector
    Int { width: usize },
    /// Floating point
    Float { width: usize, name: String },
    /// Array type
    Array { element: Box<Type>, length: usize },
    /// Struct type, which can be anonymous and/or opaque
    Struct {
        name: Option<String>,
        fields: Option<Vec<Type>>,
    },
    /// Function type, which can include variadic arguments
    Function {
        params: Vec<Type>,
        variadic: bool,
        ret: Box<Type>,
    },
    /// Pointer type
    Pointer { address_space: usize },
    /// SIMD vector type
    Vector {
        element: Box<Type>,
        fixed: bool,
        length: usize,
    },
    /// Target extension type
    Extension { name: String, params: Vec<Type> },
    /// Typed pointer
    TypedPointer {
        pointee: Box<Type>,
        address_space: usize,
    },
    /// Label type
    Label,
    /// Token type
    Token,
    /// Metadata type
    Metadata,
}

/// User-defined struct (high-level) to the module
#[derive(Serialize, Deserialize)]
pub struct UserDefinedStruct {
    pub name: Option<String>,
    pub fields: Option<Vec<Type>>,
}
