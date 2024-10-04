use std::error::Error;
use std::fmt::{Display, Formatter};

/// A list of operations not supported
#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum Unsupported {
    ModuleLevelAssembly,
    InlineAssembly,
    CallBranch,
    GlobalAlias,
    GlobalMarker,
    FloatingPointOrdering,
    VectorOfPointers,
    ScalableVector,
    VectorBitcast,
    VariadicArguments,
    ArchSpecificExtension,
    TypedPointer,
    ThreadLocalStorage,
    WeakGlobalVariable,
    WeakFunction,
    HugeConstAggregate,
    PointerAddressSpace,
    OutOfBoundConstantGEP,
    InterfaceResolver,
    AnonymousFunction,
    AnonymousGlobalVariable,
    OpaqueType,
    IntrinsicsPreAllocated,
    IntrinsicsConvergence,
    IntrinsicsCoroutine,
    IntrinsicsEH,
    IntrinsicsGC,
    AtomicInstruction,
    WindowsEH,
    MetadataSystem,
}

impl Display for Unsupported {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ModuleLevelAssembly => {
                write!(f, "module-level assembly")
            }
            Self::InlineAssembly => {
                write!(f, "inline assembly")
            }
            Self::CallBranch => {
                write!(f, "call branch")
            }
            Self::GlobalAlias => {
                write!(f, "global alias")
            }
            Self::GlobalMarker => {
                write!(f, "markers for global values")
            }
            Self::FloatingPointOrdering => {
                write!(f, "floating point ordered comparison")
            }
            Self::VectorOfPointers => {
                write!(f, "vector of pointers")
            }
            Self::ScalableVector => {
                write!(f, "scalable vector of non-fixed size")
            }
            Self::VectorBitcast => {
                write!(f, "bitcast among vector and scalar")
            }
            Self::VariadicArguments => {
                write!(f, "variadic arguments")
            }
            Self::ArchSpecificExtension => {
                write!(f, "architecture-specific extension")
            }
            Self::TypedPointer => {
                write!(f, "typed pointer")
            }
            Self::ThreadLocalStorage => {
                write!(f, "thread-local storage")
            }
            Self::WeakGlobalVariable => {
                write!(f, "weak definition for global variable")
            }
            Self::WeakFunction => {
                write!(f, "weak definition for function")
            }
            Self::HugeConstAggregate => {
                write!(f, "huge constant aggregates")
            }
            Self::PointerAddressSpace => {
                write!(f, "address space of a pointer")
            }
            Self::OutOfBoundConstantGEP => {
                write!(f, "intentional out-of-bound GEP on constant")
            }
            Self::InterfaceResolver => {
                write!(f, "load-time interface resolving")
            }
            Self::AnonymousFunction => {
                write!(f, "anonymous function")
            }
            Self::AnonymousGlobalVariable => {
                write!(f, "anonymous global variable")
            }
            Self::OpaqueType => {
                write!(f, "opaque type")
            }
            Self::IntrinsicsPreAllocated => {
                write!(f, "llvm.call.preallocated.*")
            }
            Self::IntrinsicsConvergence => {
                write!(f, "llvm.experimental.convergence.*")
            }
            Self::IntrinsicsCoroutine => {
                write!(f, "llvm.coro.*")
            }
            Self::IntrinsicsEH => {
                write!(f, "llvm.eh.exceptionpointer.*")
            }
            Self::IntrinsicsGC => {
                write!(f, "llvm.experimental.gc.*")
            }
            Self::AtomicInstruction => {
                write!(f, "atomic instruction")
            }
            Self::WindowsEH => {
                write!(f, "exception handling on windows")
            }
            Self::MetadataSystem => {
                write!(f, "metadata system")
            }
        }
    }
}

/// A set of tools
#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum Tool {
    ClangCompile,
    LLVMDis,
    LLVMLink,
    OptVerify,
    OptPipeline(String),
}

impl Display for Tool {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClangCompile => write!(f, "clang|-c|"),
            Self::LLVMDis => write!(f, "llvm-dis"),
            Self::LLVMLink => write!(f, "llvm-link"),
            Self::OptVerify => write!(f, "opt|-verify|"),
            Self::OptPipeline(s) => write!(f, "opt|{}|", s),
        }
    }
}

/// A custom error message for the analysis engine
#[derive(Debug, Clone)]
pub enum EngineError {
    /// Error during the compilation of the input
    CompilationError(Tool, String),
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
            Self::CompilationError(tool, msg) => {
                write!(f, "[libra::compilation] {}: {}", tool, msg)
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
