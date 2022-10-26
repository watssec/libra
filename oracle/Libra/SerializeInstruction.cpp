#include "Serializer.h"

namespace libra {

json::Object FunctionSerializationContext::serialize_instruction(
    const Instruction &inst) const {

  json::Object result;
  result["ty"] = serialize_type(*inst.getType());
  result["index"] = this->get_instruction(inst);
  result["repr"] = this->serialize_inst(inst);
  return result;
}

json::Object
FunctionSerializationContext::serialize_inst(const Instruction &inst) const {
  json::Object result;

  // memory
  if (isa<AllocaInst>(inst)) {
    result["Alloca"] = serialize_inst_alloca(cast<AllocaInst>(inst));
  }

  // terminators
  else if (isa<UnreachableInst>(inst)) {
    result["Unreachable"] = json::Value(nullptr);
  }

  // should have exhausted all valid cases
  else {
    LOG->fatal("unknown instruction: {0}", inst);
  }

  return result;
}

json::Object FunctionSerializationContext::serialize_inst_alloca(
    const AllocaInst &inst) const {
  json::Object result;
  result["allocated_type"] = serialize_type(*inst.getAllocatedType());
  if (inst.isArrayAllocation()) {
    result["size"] = this->serialize_value(*inst.getArraySize());
  }
  return result;
}

} // namespace libra