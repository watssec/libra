#include "Adapted.h"
#include "Shared/Logger.h"

namespace hise::ir::adapted {

Function::Function(const llvm::Function &f) {
  // set basics
  name_ = f.getName();
}

Module::Module(const llvm::Module &m) {
  // check validity of the module
  std::string message;
  raw_string_ostream ostream(message);

  bool broken_debug_info = false;
  bool verified = llvm::verifyModule(broken_debug_info, m, &ostream);
  if (!verified) {
    LOG->error("Corrupted LLVM module\n{0}", message);
    llvm_unreachable("Corrupted LLVM module");
  }
  if (broken_debug_info) {
    LOG->warning("Corrupted debug information LLVM module\n{0}", message);
  }

  // set basics
  name_ = m.getName();

  // convert functions
  for (const auto &f : m) {
    auto adapted = Function(f);
    auto result = functions_.emplace(adapted.name_, adapted);
    if (!result.second) {
      LOG->error("Duplicated function definition: {0}", adapted.name_);
    }
  }
}

} // namespace hise::ir::adapted