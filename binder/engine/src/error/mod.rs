use std::error::Error;
use std::fmt::{Display, Formatter};

/// A list of operations not supported
#[derive(Debug, Clone)]
pub enum Unsupported {
    ModuleLevelAssembly,
    GlobalAlias,
    FloatingPoint,
    Vectorization,
    VariadicArguments,
    ArchSpecificExtension,
    OpaqueStructDefinition,
    OpaquePointerType,
    ThreadLocalStorage,
    PointerAddressSpace,
    OutOfBoundConstantGEP,
}

impl Display for Unsupported {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ModuleLevelAssembly => {
                write!(f, "module-level assembly")
            }
            Self::GlobalAlias => {
                write!(f, "global alias")
            }
            Self::FloatingPoint => {
                write!(f, "floating point")
            }
            Self::Vectorization => {
                write!(f, "SIMD vectorization")
            }
            Self::VariadicArguments => {
                write!(f, "variadic arguments")
            }
            Self::ArchSpecificExtension => {
                write!(f, "architecture-specific extension")
            }
            Self::OpaqueStructDefinition => {
                write!(f, "opaque struct definition")
            }
            Self::OpaquePointerType => {
                write!(f, "opaque pointer type")
            }
            Self::ThreadLocalStorage => {
                write!(f, "thread-local storage")
            }
            Self::PointerAddressSpace => {
                write!(f, "address space of a pointer")
            }
            Self::OutOfBoundConstantGEP => {
                write!(f, "intentional out-of-bound GEP on constant")
            }
        }
    }
}

/// A custom error message for the analysis engine
#[derive(Debug, Clone)]
pub enum EngineError {
    /// Error during the compilation of the input
    CompilationError(String),
    /// Error during the loading of a compiled LLVM module
    LLVMLoadingError(String),
    /// Invalid assumption made about the program
    InvalidAssumption(String),
    /// Operation not supported yet
    NotSupportedYet(Unsupported),
    /// Invariant violation
    InvariantViolation(String),
}

pub type EngineResult<T> = Result<T, EngineError>;

impl Display for EngineError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CompilationError(msg) => {
                write!(f, "[libra::compilation] {}", msg)
            }
            Self::LLVMLoadingError(msg) => {
                write!(f, "[libra::loading] {}", msg)
            }
            Self::InvalidAssumption(msg) => {
                write!(f, "[libra::assumption] {}", msg)
            }
            Self::NotSupportedYet(item) => {
                write!(f, "[libra::unsupported] {}", item)
            }
            Self::InvariantViolation(msg) => {
                write!(f, "[libra::invariant] {}", msg)
            }
        }
    }
}

impl Error for EngineError {}
