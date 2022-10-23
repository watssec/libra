#include "Serializer.h"

namespace libra {

json::Object serialize_module(const Module &module) {
  json::Object result;

  // module level info
  result["name"] = module.getModuleIdentifier();
  result["asm"] = module.getModuleInlineAsm();

  // user-defined struct types
  json::Array structs;
  for (const auto *ty_def : module.getIdentifiedStructTypes()) {
    structs.push_back(serialize_type_struct(*ty_def));
  }
  result["structs"] = std::move(structs);

  // globals
  // TODO

  // functions
  // TODO

  // done
  return result;
}

} // namespace libra