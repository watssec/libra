#include "Serializer.h"

namespace libra {

json::Object FunctionSerializationContext::serialize_instruction(
    const Instruction &inst) const {

  json::Object result;
  result["ty"] = serialize_type(*inst.getType());
  result["index"] = this->get_instruction(inst);
  if (inst.hasName()) {
    result["name"] = inst.getName();
  }
  result["repr"] = this->serialize_inst(inst);
  return result;
}

json::Object
FunctionSerializationContext::serialize_inst(const Instruction &inst) const {
  json::Object result;

  // memory
  if (isa<AllocaInst>(inst)) {
    result["Alloca"] = serialize_inst_alloca(cast<AllocaInst>(inst));
  } else if (isa<LoadInst>(inst)) {
    result["Load"] = serialize_inst_load(cast<LoadInst>(inst));
  } else if (isa<StoreInst>(inst)) {
    result["Load"] = serialize_inst_store(cast<StoreInst>(inst));
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

json::Object
FunctionSerializationContext::serialize_inst_load(const LoadInst &inst) const {
  json::Object result;
  result["pointee_type"] = serialize_type(*inst.getPointerOperandType());
  result["pointer"] = this->serialize_value(*inst.getPointerOperand());
  result["address_space"] = inst.getPointerAddressSpace();
  return result;
}

json::Object FunctionSerializationContext::serialize_inst_store(
    const StoreInst &inst) const {
  json::Object result;
  result["pointee_type"] = serialize_type(*inst.getPointerOperandType());
  result["pointer"] = this->serialize_value(*inst.getPointerOperand());
  result["value"] = this->serialize_value(*inst.getValueOperand());
  result["address_space"] = inst.getPointerAddressSpace();
  return result;
}

} // namespace libra