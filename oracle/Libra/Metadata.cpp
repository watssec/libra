#include "Metadata.h"

namespace libra {

bool is_debug_function(const Function &func) {
  const auto intrinsic_id = func.getIntrinsicID();
  return intrinsic_id != Intrinsic::not_intrinsic &&
         isDbgInfoIntrinsic(intrinsic_id);
}

bool is_debug_instruction(const Instruction &inst) {
  if (isa<IntrinsicInst>(inst)) {
    const auto &intrinsic_inst = cast<IntrinsicInst>(inst);
    if (isDbgInfoIntrinsic(intrinsic_inst.getIntrinsicID())) {
      return true;
    }
  }
  return false;
}

bool is_intrinsic_function(const Function &func) {
  if (func.isIntrinsic()) {
    return true;
  }
  const auto intrinsic_id = func.getIntrinsicID();
  if (intrinsic_id != Intrinsic::not_intrinsic) {
    return true;
  }
  if (func.hasName() && func.getName().starts_with("llvm.")) {
    return true;
  }
  return false;
}

} // namespace libra