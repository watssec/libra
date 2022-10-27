#ifndef LIBRA_METADATA_H
#define LIBRA_METADATA_H

#include "Config.hpp"
#include "Deps.h"
#include "Logger.h"

namespace libra {

bool is_debug_function(const Function &func);
bool is_debug_instruction(const Instruction &inst);

} // namespace libra

#endif // LIBRA_METADATA_H
