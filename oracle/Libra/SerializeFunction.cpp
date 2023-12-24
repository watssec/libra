#include "Serializer.h"

namespace libra {

json::Object serialize_function(const Function &func) {
  json::Object result;

  // first label the blocks, instructions, and arguments
  FunctionSerializationContext ctxt(&func);
  for (const auto &arg : func.args()) {
    ctxt.add_argument(arg);
  }
  for (const auto &block : func) {
    ctxt.add_block(block);
    for (const auto &inst : block) {
      ctxt.add_instruction(inst);
    }
  }

  // set the context
  if (current_function != nullptr) {
    LOG->fatal("already in function context");
  }
  current_function = &ctxt;

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

  // deserialize the block
  json::Array blocks;
  for (const auto &block : func) {
    blocks.push_back(ctxt.serialize_block(block));
  }
  result["blocks"] = std::move(blocks);

  // reset the cursor
  current_function = nullptr;

  return result;
}

json::Object serialize_parameter(const Argument &param) {
  json::Object result;

  // basics
  result["ty"] = serialize_type(*param.getType());
  if (param.hasName()) {
    result["name"] = param.getName();
  }

  // argument-specific attrs
  if (param.hasByValAttr()) {
    result["by_val"] = serialize_type(*param.getParamByValType());
  }
  if (param.hasByRefAttr()) {
    result["by_ref"] = serialize_type(*param.getParamByRefType());
  }
  if (param.hasPreallocatedAttr()) {
    result["pre_allocated"] =
        serialize_type(*param.getPointeeInMemoryValueType());
  }
  if (param.hasStructRetAttr()) {
    result["struct_ret"] = serialize_type(*param.getParamStructRetType());
  }
  if (param.hasInAllocaAttr()) {
    result["in_alloca"] = serialize_type(*param.getParamInAllocaType());
  }

  // opaque pointer
  if (param.hasAttribute(Attribute::AttrKind::ElementType)) {
    result["element_type"] = serialize_type(
        *param.getAttribute(Attribute::AttrKind::ElementType).getValueAsType());
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