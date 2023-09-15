use crate::error::{EngineError, EngineResult, Unsupported};

pub fn filter_intrinsics(name: &str) -> EngineResult<()> {
    match name.strip_prefix("llvm.experimental.gc.") {
        None => (),
        Some(_) => {
            // NOTE: involves `token` type
            return Err(EngineError::NotSupportedYet(
                Unsupported::IntrinsicsExperimentalGC,
            ));
        }
    }
    match name.strip_prefix("llvm.call.preallocated.") {
        None => (),
        Some(_) => {
            // NOTE: involves `token` type
            return Err(EngineError::NotSupportedYet(
                Unsupported::IntrinsicsPreAllocated,
            ));
        }
    }
    Ok(())
}
