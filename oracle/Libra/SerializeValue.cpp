#include "Serializer.h"

namespace libra {

json::Object
FunctionSerializationContext::serialize_value(const Value &val) const {
  json::Object result;
  if (isa<Argument>(val)) {
    result["Argument"] = serialize_value_argument(cast<Argument>(val));
  } else if (isa<Constant>(val)) {
    result["Constant"] = serialize_constant(cast<Constant>(val));
  } else if (isa<Instruction>(val)) {
    result["Instruction"] = serialize_value_instruction(cast<Instruction>(val));
  } else if (isa<BasicBlock>(val)) {
    result["Label"] = serialize_value_block(cast<BasicBlock>(val));
  } else if (isa<MetadataAsValue>(val)) {
    // TODO: metadata system is not ready
    result["Metadata"] = json::Value(nullptr);
  } else if (isa<InlineAsm>(val)) {
    LOG->fatal("unexpected asm as value");
  } else if (isa<Operator>(val)) {
    LOG->fatal("unexpected operator as value");
  } else if (isa<MemoryAccess>(val)) {
    LOG->fatal("unexpected memory SSA as value");
  } else {
    LOG->fatal("unknown value type: {0}", val);
  }
  return result;
}

json::Object FunctionSerializationContext::serialize_value_argument(
    const Argument &arg) const {
  json::Object result;
  result["ty"] = serialize_type(*arg.getType());
  result["index"] = get_argument(arg);
  return result;
}

json::Object FunctionSerializationContext::serialize_value_block(
    const BasicBlock &block) const {
  // sanity checks
  const auto *func = block.getParent();
  if (!func->hasName()) {
    LOG->fatal("block address referring to an unnamed function");
  }

  // lookup context
  const auto iter = contexts.find(func);
  if (iter == contexts.cend()) {
    LOG->fatal("function context not ready");
  }
  const auto &ctxt = iter->second;

  // dump the result
  json::Object result;
  result["func"] = func->getName();
  result["block"] = ctxt.get_block(block);
  return result;
}

json::Object FunctionSerializationContext::serialize_value_instruction(
    const Instruction &inst) const {
  json::Object result;
  result["ty"] = serialize_type(*inst.getType());
  result["index"] = get_instruction(inst);
  return result;
}

} // namespace libra