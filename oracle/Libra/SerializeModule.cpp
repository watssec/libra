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
  json::Array global_vars;
  for (const auto &global_var : module.globals()) {
    global_vars.push_back(serialize_global_variable(global_var));
  }
  result["global_variables"] = std::move(global_vars);

  // functions
  // TODO

  // done
  return result;
}

} // namespace libra