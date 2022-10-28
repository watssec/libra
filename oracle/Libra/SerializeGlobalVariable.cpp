#include "Serializer.h"

namespace libra {

json::Object serialize_global_variable(const GlobalVariable &gvar) {
  json::Object result;

  // basics
  if (gvar.hasName()) {
    result["name"] = gvar.getName();
  } else {
    LOG->error("unnamed global variable: {0}", gvar);
  }
  result["ty"] = serialize_type(*gvar.getValueType());

  // attributes
  result["is_extern"] = gvar.isExternallyInitialized();
  result["is_const"] = gvar.isConstant();
  result["is_defined"] = !gvar.isDeclaration();
  result["is_exact"] = gvar.isDefinitionExact();
  result["is_thread_local"] = gvar.isThreadLocal();
  result["address_space"] = gvar.getAddressSpace();
  // TODO: additional attributes or metadata?

  // initializer
  if (gvar.hasInitializer()) {
    result["initializer"] = serialize_constant(*gvar.getInitializer());
  }

  return result;
}

} // namespace libra