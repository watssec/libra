#include "Serializer.h"

namespace libra {

json::Object serialize_function(const Function &func) {
  json::Object result;

  // basics
  if (func.hasName()) {
    result["name"] = func.getName();
  } else {
    LOG->error("unnamed function: {0}", func);
  }
  result["ty"] = serialize_type(*func.getFunctionType());

  // attributes
  if (func.isIntrinsic()) {
    result["intrinsic"] = func.getIntrinsicID();
  }
  result["is_exact"] = !func.isDeclaration() && func.isDefinitionExact();
  // TODO: additional attributes or metadata?

  // parameters
  json::Array params;
  for (const auto &param : func.args()) {
    params.push_back(serialize_parameter(param));
  }
  result["params"] = std::move(params);

  // body
  // TODO

  return result;
}

json::Object serialize_parameter(const Argument &param) {
  json::Object result;
  if (param.hasName()) {
    result["name"] = param.getName();
  }
  result["ty"] = serialize_type(*param.getType());
  return result;
}

} // namespace libra