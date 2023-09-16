use crate::error::{EngineError, EngineResult, Unsupported};

pub fn filter_intrinsics(name: &str) -> EngineResult<()> {
    // pre-allocated args
    match name.strip_prefix("llvm.call.preallocated.") {
        None => (),
        Some(_) => {
            return Err(EngineError::NotSupportedYet(
                Unsupported::IntrinsicsPreAllocated,
            ));
        }
    }

    // convergence
    match name.strip_prefix("llvm.experimental.convergence.") {
        None => (),
        Some(_) => {
            return Err(EngineError::NotSupportedYet(
                Unsupported::IntrinsicsConvergence,
            ));
        }
    }

    // coroutine
    match name.strip_prefix("llvm.coro.") {
        None => (),
        Some(_) => {
            return Err(EngineError::NotSupportedYet(
                Unsupported::IntrinsicsCoroutine,
            ));
        }
    }

    // garbage collection
    match name.strip_prefix("llvm.experimental.gc.") {
        None => (),
        Some(_) => {
            return Err(EngineError::NotSupportedYet(Unsupported::IntrinsicsGC));
        }
    }

    // exception handling
    match name.strip_prefix("llvm.eh.") {
        None => (),
        Some(_) => {
            return Err(EngineError::NotSupportedYet(Unsupported::IntrinsicsEH));
        }
    }

    // other intrinsics are okay
    Ok(())
}
