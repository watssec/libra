#include "Serializer.h"

namespace libra {

json::Object
FunctionSerializationContext::serialize_value(const Value &val) const {
  json::Object result;
  if (isa<Argument>(val)) {
    result["Argument"] = this->serialize_value_argument(cast<Argument>(val));
  } else if (isa<Constant>(val)) {
    result["Constant"] = serialize_constant(cast<Constant>(val));
  } else if (isa<Instruction>(val)) {
    result["Instruction"] =
        this->serialize_value_instruction(cast<Instruction>(val));
  } else if (isa<MetadataAsValue>(val)) {
    // TODO: metadata system is not ready
    result["Metadata"] = json::Value(nullptr);
  } else if (isa<InlineAsm>(val)) {
    LOG->fatal("unexpected asm as value");
  } else if (isa<Operator>(val)) {
    LOG->fatal("unexpected operator as value");
  } else if (isa<MemoryAccess>(val)) {
    LOG->fatal("unexpected memory SSA as value");
  } else if (isa<BasicBlock>(val)) {
    LOG->fatal("unexpected block as value");
  } else {
    LOG->fatal("unknown value type: {0}", val);
  }
  return result;
}

json::Object FunctionSerializationContext::serialize_value_argument(
    const Argument &arg) const {
  json::Object result;
  result["ty"] = serialize_type(*arg.getType());
  result["index"] = this->get_argument(arg);
  return result;
}

json::Object FunctionSerializationContext::serialize_value_instruction(
    const Instruction &inst) const {
  json::Object result;
  result["ty"] = serialize_type(*inst.getType());
  result["index"] = this->get_instruction(inst);
  return result;
}

} // namespace libra