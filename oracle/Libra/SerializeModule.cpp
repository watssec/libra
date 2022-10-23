#include "Serializer.h"

namespace libra {

json::Object serialize_module(const Module &module) {
  json::Object result;

  // module level info
  result["name"] = module.getModuleIdentifier();
  result["asm"] = module.getModuleInlineAsm();

  // user-defined struct types
  for (const auto *ty_def : module.getIdentifiedStructTypes()) {
    serialize_type_struct(*ty_def);
  }
  // TODO

  // globals
  // TODO

  // functions
  // TODO

  // done
  return result;
}

} // namespace libra