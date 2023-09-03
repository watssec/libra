#ifndef LIBRA_METADATA_H
#define LIBRA_METADATA_H

#include "Deps.h"
#include "Logger.h"

namespace libra {

bool is_debug_function(const Function &func);
bool is_debug_instruction(const Instruction &inst);

/// TODO: this function is introduced as we notice a weird behavior of LLVM
/// where some intrinsic functions (e.g., llvm.memset.*) are not marked as
/// intrinsic nor assigned an intrinsic ID.
bool is_intrinsic_function(const Function &func);

} // namespace libra

#endif // LIBRA_METADATA_H
