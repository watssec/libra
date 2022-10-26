#include "Serializer.h"

namespace libra {

json::Object serialize_instruction(
    const Instruction &inst,
    const std::map<const BasicBlock *, uint64_t> &block_labels,
    const std::map<const Instruction *, uint64_t> &inst_labels) {

  json::Object result;
  result["ty"] = serialize_type(*inst.getType());
  result["index"] = inst_labels.at(&inst);
  result["repr"] = serialize_inst(inst);
  return result;
}

json::Object serialize_inst(const Instruction &inst) {
  json::Object result;

  // memory
  if (isa<AllocaInst>(inst)) {
    result["Alloca"] = serialize_inst_alloca(cast<AllocaInst>(inst));
  }

  // terminators
  else if (isa<UnreachableInst>(inst)) {
    result["Unreachable"] =
        serialize_inst_unreachable(cast<UnreachableInst>(inst));
  }

  // should have exhausted all valid cases
  else {
    LOG->fatal("unknown instruction: {0}", inst);
  }

  return result;
}

json::Value serialize_inst_alloca(const AllocaInst &inst) {
  json::Object result;
  result["allocated_type"] = serialize_type(*inst.getAllocatedType());
  if (inst.isArrayAllocation()) {
    result["size"] = serialize_value(*inst.getArraySize());
  }
  return result;
}

json::Value serialize_inst_unreachable(const UnreachableInst &inst) {
  return {nullptr};
}

} // namespace libra