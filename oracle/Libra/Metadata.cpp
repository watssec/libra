#include "Metadata.h"

namespace libra {

bool is_debug_function(const Function &func) {
  return func.isIntrinsic() && isDbgInfoIntrinsic(func.getIntrinsicID());
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

} // namespace libra