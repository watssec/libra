use crate::error::{EngineError, EngineResult, Unsupported};

pub fn filter_intrinsics(name: &str) -> EngineResult<()> {
    // NOTE: involves `token` type
    match name.strip_prefix("llvm.call.preallocated.") {
        None => (),
        Some(_) => {
            return Err(EngineError::NotSupportedYet(
                Unsupported::IntrinsicsPreAllocated,
            ));
        }
    }
    match name.strip_prefix("llvm.experimental.convergence.") {
        None => (),
        Some(_) => {
            return Err(EngineError::NotSupportedYet(
                Unsupported::IntrinsicsConvergence,
            ));
        }
    }
    match name.strip_prefix("llvm.experimental.gc.") {
        None => (),
        Some(_) => {
            return Err(EngineError::NotSupportedYet(Unsupported::IntrinsicsGC));
        }
    }
    Ok(())
}
