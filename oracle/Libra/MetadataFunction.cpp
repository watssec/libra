#include "Metadata.h"

namespace libra {

bool is_debug_function(const Function &func) {
  return func.isIntrinsic() && isDbgInfoIntrinsic(func.getIntrinsicID());
}

} // namespace libra