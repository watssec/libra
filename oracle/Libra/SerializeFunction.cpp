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
  result["is_defined"] = !func.isDeclaration();
  result["is_exact"] = func.isDefinitionExact();
  result["is_intrinsic"] = is_intrinsic_function(func);
  // TODO: additional attributes or metadata?

  // parameters
  json::Array params;
  for (const auto &param : func.args()) {
    params.push_back(serialize_parameter(param));
  }
  result["params"] = std::move(params);

  // first label the blocks, instructions, and arguments
  FunctionSerializationContext ctxt;
  for (const auto &block : func) {
    ctxt.add_block(block);
    for (const auto &inst : block) {
      ctxt.add_instruction(inst);
    }
  }
  for (const auto &arg : func.args()) {
    ctxt.add_argument(arg);
  }

  // deserialize the block
  json::Array blocks;
  for (const auto &block : func) {
    blocks.push_back(ctxt.serialize_block(block));
  }
  result["blocks"] = std::move(blocks);

  return result;
}

json::Object serialize_parameter(const Argument &param) {
  json::Object result;
  result["ty"] = serialize_type(*param.getType());
  if (param.hasName()) {
    result["name"] = param.getName();
  }
  return result;
}

json::Object
FunctionSerializationContext::serialize_block(const BasicBlock &block) const {
  json::Object result;

  // basics
  result["label"] = get_block(block);
  if (block.hasName()) {
    result["name"] = block.getName();
  }

  // body
  const auto *term = block.getTerminator();

  json::Array body;
  for (const auto &inst : block) {
    // handle terminator separately
    if (term == &inst) {
      continue;
    }
    // handle debug instructions separately
    if (is_debug_instruction(inst)) {
      continue;
    }
    body.push_back(serialize_instruction(inst));
  }
  result["body"] = std::move(body);

  // terminator
  result["terminator"] = serialize_instruction(*term);

  return result;
}

} // namespace libra