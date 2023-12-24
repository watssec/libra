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
  if (current_function == nullptr ||
      block.getParent() != current_function->func_) {
    LOG->fatal("block address out of scope");
  }
  if (!current_function->func_->hasName()) {
    LOG->fatal("block address referring to an unnamed function");
  }

  json::Object result;
  result["func"] = current_function->func_->getName();
  result["block"] = current_function->get_block(block);
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