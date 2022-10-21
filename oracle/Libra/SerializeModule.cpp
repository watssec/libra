#include "Serializer.h"

namespace libra {

json::Object serialize_module(const Module &module) {
  json::Object result;

  // module level info
  result["name"] = module.getModuleIdentifier();
  result["asm"] = module.getModuleInlineAsm();

  // user-defined structs
  // TODO

  // globals
  // TODO

  // functions
  // TODO

  // done
  return result;
}

} // namespace libra