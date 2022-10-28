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
    result["Store"] = serialize_inst_store(cast<StoreInst>(inst));
  }

  // call
  else if (isa<CallInst>(inst)) {
    const auto &call_inst = cast<CallInst>(inst);
    if (isa<IntrinsicInst>(call_inst)) {
      result["Intrinsic"] =
          serialize_inst_call_intrinsic(cast<IntrinsicInst>(call_inst));
    } else if (call_inst.isInlineAsm()) {
      result["Asm"] = serialize_inst_call_asm(call_inst);
    } else if (call_inst.isIndirectCall()) {
      result["CallIndirect"] = serialize_inst_call_indirect(call_inst);
    } else {
      result["CallDirect"] = serialize_inst_call_direct(call_inst);
    }
  }

  // terminators
  else if (isa<ReturnInst>(inst)) {
    result["Return"] = serialize_inst_return(cast<ReturnInst>(inst));
  } else if (isa<UnreachableInst>(inst)) {
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

[[nodiscard]] json::Object
FunctionSerializationContext::serialize_inst_call_asm(
    const CallInst &inst) const {
  json::Object result;
  result["asm"] =
      serialize_inline_asm(*cast<InlineAsm>(inst.getCalledOperand()));

  json::Array args;
  for (const auto &arg : inst.args()) {
    args.push_back(serialize_value(*arg.get()));
  }
  result["args"] = std::move(args);
  return result;
}

[[nodiscard]] json::Object
FunctionSerializationContext::serialize_inst_call_direct(
    const CallInst &inst) const {
  json::Object result;
  result["callee"] = serialize_value(*inst.getCalledOperand());
  result["target_type"] = serialize_type(*inst.getFunctionType());

  json::Array args;
  for (const auto &arg : inst.args()) {
    args.push_back(serialize_value(*arg.get()));
  }
  result["args"] = std::move(args);
  return result;
}

[[nodiscard]] json::Object
FunctionSerializationContext::serialize_inst_call_indirect(
    const CallInst &inst) const {
  json::Object result;
  result["callee"] = serialize_value(*inst.getCalledOperand());
  result["target_type"] = serialize_type(*inst.getFunctionType());

  json::Array args;
  for (const auto &arg : inst.args()) {
    args.push_back(serialize_value(*arg.get()));
  }
  result["args"] = std::move(args);
  return result;
}

[[nodiscard]] json::Object
FunctionSerializationContext::serialize_inst_call_intrinsic(
    const IntrinsicInst &inst) const {
  json::Object result;
  result["callee"] = serialize_value(*inst.getCalledOperand());
  result["target_type"] = serialize_type(*inst.getFunctionType());

  json::Array args;
  for (const auto &arg : inst.args()) {
    args.push_back(serialize_value(*arg.get()));
  }
  result["args"] = std::move(args);
  return result;
}

json::Object FunctionSerializationContext::serialize_inst_return(
    const ReturnInst &inst) const {
  json::Object result;
  const auto *rv = inst.getReturnValue();
  if (rv != nullptr) {
    result["value"] = this->serialize_value(*inst.getReturnValue());
  }
  return result;
}

} // namespace libra