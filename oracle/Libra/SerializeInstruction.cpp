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

  // unary, binary, comparison, and cast
  else if (isa<UnaryOperator>(inst)) {
    result["Unary"] = serialize_inst_unary_operator(cast<UnaryOperator>(inst));
  } else if (isa<BinaryOperator>(inst)) {
    result["Binary"] =
        serialize_inst_binary_operator(cast<BinaryOperator>(inst));
  } else if (isa<CmpInst>(inst)) {
    result["Compare"] = serialize_inst_compare(cast<CmpInst>(inst));
  } else if (isa<CastInst>(inst)) {
    result["Cast"] = serialize_inst_cast(cast<CastInst>(inst));
  }

  // pointer arithmetic
  else if (isa<GetElementPtrInst>(inst)) {
    result["GEP"] = serialize_inst_gep(cast<GetElementPtrInst>(inst));
  }

  // choice
  else if (isa<PHINode>(inst)) {
    result["Phi"] = serialize_inst_phi(cast<PHINode>(inst));
  } else if (isa<SelectInst>(inst)) {
    result["ITE"] = serialize_inst_ite(cast<SelectInst>(inst));
  }

  // TODO: concurrency instructions
  else if (isa<FenceInst>(inst)) {
    result["Fence"] = json::Value(nullptr);
  } else if (isa<AtomicCmpXchgInst>(inst)) {
    result["AtomicCmpXchg"] = json::Value(nullptr);
  } else if (isa<AtomicRMWInst>(inst)) {
    result["AtomicRMW"] = json::Value(nullptr);
  }

  // terminators
  else if (isa<ReturnInst>(inst)) {
    result["Return"] = serialize_inst_return(cast<ReturnInst>(inst));
  } else if (isa<BranchInst>(inst)) {
    result["Branch"] = serialize_inst_branch(cast<BranchInst>(inst));
  } else if (isa<SwitchInst>(inst)) {
    result["Switch"] = serialize_inst_switch(cast<SwitchInst>(inst));
  } else if (isa<UnreachableInst>(inst)) {
    result["Unreachable"] = json::Value(nullptr);
  }

  // TODO: indirect branch
  else if (isa<IndirectBrInst>(inst)) {
    result["IndirectJump"] = json::Value(nullptr);
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
  result["pointee_type"] = serialize_type(*inst.getType());
  result["pointer"] = this->serialize_value(*inst.getPointerOperand());
  result["address_space"] = inst.getPointerAddressSpace();
  return result;
}

json::Object FunctionSerializationContext::serialize_inst_store(
    const StoreInst &inst) const {
  json::Object result;
  result["pointee_type"] = serialize_type(*inst.getValueOperand()->getType());
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

[[nodiscard]] json::Object
FunctionSerializationContext::serialize_inst_unary_operator(
    const UnaryOperator &inst) const {
  json::Object result;

  switch (inst.getOpcode()) {
  case Instruction::UnaryOps::FNeg: {
    result["opcode"] = "fneg";
    break;
  }
  case Instruction::UnaryOpsEnd: {
    LOG->fatal("unexpected end of unary ops");
  }
  }

  result["operand"] = serialize_value(*inst.getOperand(0));
  return result;
}

[[nodiscard]] json::Object
FunctionSerializationContext::serialize_inst_binary_operator(
    const BinaryOperator &inst) const {
  json::Object result;

  switch (inst.getOpcode()) {
  case Instruction::BinaryOps::Add: {
    result["opcode"] = "add";
    break;
  }
  case Instruction::BinaryOps::FAdd: {
    result["opcode"] = "fadd";
    break;
  }
  case Instruction::BinaryOps::Sub: {
    result["opcode"] = "sub";
    break;
  }
  case Instruction::BinaryOps::FSub: {
    result["opcode"] = "fsub";
    break;
  }
  case Instruction::BinaryOps::Mul: {
    result["opcode"] = "mul";
    break;
  }
  case Instruction::BinaryOps::FMul: {
    result["opcode"] = "fmul";
    break;
  }
  case Instruction::BinaryOps::UDiv: {
    result["opcode"] = "udiv";
    break;
  }
  case Instruction::BinaryOps::SDiv: {
    result["opcode"] = "sdiv";
    break;
  }
  case Instruction::BinaryOps::FDiv: {
    result["opcode"] = "fdiv";
    break;
  }
  case Instruction::BinaryOps::URem: {
    result["opcode"] = "urem";
    break;
  }
  case Instruction::BinaryOps::SRem: {
    result["opcode"] = "srem";
    break;
  }
  case Instruction::BinaryOps::FRem: {
    result["opcode"] = "frem";
    break;
  }
  case Instruction::BinaryOps::Shl: {
    result["opcode"] = "shl";
    break;
  }
  case Instruction::BinaryOps::LShr: {
    result["opcode"] = "lshr";
    break;
  }
  case Instruction::BinaryOps::AShr: {
    result["opcode"] = "ashr";
    break;
  }
  case Instruction::BinaryOps::And: {
    result["opcode"] = "and";
    break;
  }
  case Instruction::BinaryOps::Or: {
    result["opcode"] = "or";
    break;
  }
  case Instruction::BinaryOps::Xor: {
    result["opcode"] = "xor";
    break;
  }
  case Instruction::BinaryOpsEnd: {
    LOG->fatal("unexpected end of binary ops");
  }
  }
  // TODO: flags (NSW, NUW, Exact)? Maybe not needed?
  result["lhs"] = serialize_value(*inst.getOperand(0));
  result["rhs"] = serialize_value(*inst.getOperand(1));
  return result;
}

[[nodiscard]] json::Object FunctionSerializationContext::serialize_inst_compare(
    const CmpInst &inst) const {
  json::Object result;

  switch (inst.getPredicate()) {
  case CmpInst::Predicate::FCMP_FALSE: {
    result["predicate"] = "f_false";
    break;
  }
  case CmpInst::Predicate::FCMP_OEQ: {
    result["predicate"] = "f_oeq";
    break;
  }
  case CmpInst::Predicate::FCMP_OGT: {
    result["predicate"] = "f_ogt";
    break;
  }
  case CmpInst::Predicate::FCMP_OGE: {
    result["predicate"] = "f_oge";
    break;
  }
  case CmpInst::Predicate::FCMP_OLT: {
    result["predicate"] = "f_olt";
    break;
  }
  case CmpInst::Predicate::FCMP_OLE: {
    result["predicate"] = "f_ole";
    break;
  }
  case CmpInst::Predicate::FCMP_ONE: {
    result["predicate"] = "f_one";
    break;
  }
  case CmpInst::Predicate::FCMP_ORD: {
    result["predicate"] = "f_ord";
    break;
  }
  case CmpInst::Predicate::FCMP_UNO: {
    result["predicate"] = "f_uno";
    break;
  }
  case CmpInst::Predicate::FCMP_UEQ: {
    result["predicate"] = "f_ueq";
    break;
  }
  case CmpInst::Predicate::FCMP_UGT: {
    result["predicate"] = "f_ugt";
    break;
  }
  case CmpInst::Predicate::FCMP_UGE: {
    result["predicate"] = "f_uge";
    break;
  }
  case CmpInst::Predicate::FCMP_ULT: {
    result["predicate"] = "f_ult";
    break;
  }
  case CmpInst::Predicate::FCMP_ULE: {
    result["predicate"] = "f_ule";
    break;
  }
  case CmpInst::Predicate::FCMP_UNE: {
    result["predicate"] = "f_une";
    break;
  }
  case CmpInst::Predicate::FCMP_TRUE: {
    result["predicate"] = "f_true";
    break;
  }
  case CmpInst::Predicate::ICMP_EQ: {
    result["predicate"] = "i_eq";
    break;
  }
  case CmpInst::Predicate::ICMP_NE: {
    result["predicate"] = "i_ne";
    break;
  }
  case CmpInst::Predicate::ICMP_UGT: {
    result["predicate"] = "i_ugt";
    break;
  }
  case CmpInst::Predicate::ICMP_UGE: {
    result["predicate"] = "i_uge";
    break;
  }
  case CmpInst::Predicate::ICMP_ULT: {
    result["predicate"] = "i_ult";
    break;
  }
  case CmpInst::Predicate::ICMP_ULE: {
    result["predicate"] = "i_ule";
    break;
  }
  case CmpInst::Predicate::ICMP_SGT: {
    result["predicate"] = "i_sgt";
    break;
  }
  case CmpInst::Predicate::ICMP_SGE: {
    result["predicate"] = "i_sge";
    break;
  }
  case CmpInst::Predicate::ICMP_SLT: {
    result["predicate"] = "i_slt";
    break;
  }
  case CmpInst::Predicate::ICMP_SLE: {
    result["predicate"] = "i_sle";
    break;
  }
  case CmpInst::Predicate::BAD_FCMP_PREDICATE:
  case CmpInst::Predicate::BAD_ICMP_PREDICATE: {
    LOG->fatal("unexpected bad compare predicate");
  }
  }

  result["operand_type"] = serialize_type(*inst.getOperand(0)->getType());
  result["lhs"] = serialize_value(*inst.getOperand(0));
  result["rhs"] = serialize_value(*inst.getOperand(1));
  return result;
}

[[nodiscard]] json::Object
FunctionSerializationContext::serialize_inst_cast(const CastInst &inst) const {
  json::Object result;

  switch (inst.getOpcode()) {
  case Instruction::CastOps::Trunc: {
    result["opcode"] = "trunc";
    break;
  }
  case Instruction::CastOps::ZExt: {
    result["opcode"] = "zext";
    break;
  }
  case Instruction::CastOps::SExt: {
    result["opcode"] = "sext";
    break;
  }
  case Instruction::CastOps::FPToUI: {
    result["opcode"] = "fp_to_ui";
    break;
  }
  case Instruction::CastOps::FPToSI: {
    result["opcode"] = "fp_to_si";
    break;
  }
  case Instruction::CastOps::UIToFP: {
    result["opcode"] = "ui_to_fp";
    break;
  }
  case Instruction::CastOps::SIToFP: {
    result["opcode"] = "si_to_fp";
    break;
  }
  case Instruction::CastOps::FPTrunc: {
    result["opcode"] = "fp_trunc";
    break;
  }
  case Instruction::CastOps::FPExt: {
    result["opcode"] = "fp_ext";
    break;
  }
  case Instruction::CastOps::PtrToInt: {
    result["opcode"] = "ptr_to_int";
    break;
  }
  case Instruction::CastOps::IntToPtr: {
    result["opcode"] = "int_to_ptr";
    break;
  }
  case Instruction::CastOps::BitCast: {
    result["opcode"] = "bitcast";
    break;
  }
  case Instruction::CastOps::AddrSpaceCast: {
    result["opcode"] = "address_space_cast";
    break;
  }
  case Instruction::CastOpsEnd: {
    LOG->fatal("unexpected end of cast ops");
  }
  }

  result["src_ty"] = serialize_type(*inst.getSrcTy());
  result["dst_ty"] = serialize_type(*inst.getDestTy());
  result["operand"] = serialize_value(*inst.getOperand(0));
  return result;
}

json::Object FunctionSerializationContext::serialize_inst_gep(
    const GetElementPtrInst &inst) const {
  json::Object result;
  result["src_pointee_ty"] = serialize_type(*inst.getSourceElementType());
  result["dst_pointee_ty"] = serialize_type(*inst.getResultElementType());

  result["pointer"] = serialize_value(*inst.getPointerOperand());
  json::Array indices;
  for (const auto &idx : inst.indices()) {
    indices.push_back(serialize_value(*idx.get()));
  }
  result["indices"] = std::move(indices);

  result["address_space"] = inst.getAddressSpace();
  return result;
}

json::Object
FunctionSerializationContext::serialize_inst_phi(const PHINode &inst) const {
  json::Object result;

  json::Array blocks;
  for (const auto *block : inst.blocks()) {
    json::Object item;
    item["block"] = get_block(*block);
    item["value"] = serialize_value(*inst.getIncomingValueForBlock(block));
    blocks.push_back(std::move(item));
  }
  result["options"] = std::move(blocks);

  return result;
}

json::Object
FunctionSerializationContext::serialize_inst_ite(const SelectInst &inst) const {
  json::Object result;
  result["cond"] = serialize_value(*inst.getCondition());
  result["then_value"] = serialize_value(*inst.getTrueValue());
  result["else_value"] = serialize_value(*inst.getFalseValue());
  return result;
}

json::Object FunctionSerializationContext::serialize_inst_return(
    const ReturnInst &inst) const {
  json::Object result;
  const auto *rv = inst.getReturnValue();
  if (rv != nullptr) {
    result["value"] = serialize_value(*inst.getReturnValue());
  }
  return result;
}

json::Object FunctionSerializationContext::serialize_inst_branch(
    const BranchInst &inst) const {
  json::Object result;
  if (inst.isConditional()) {
    result["cond"] = serialize_value(*inst.getCondition());
  }
  json::Array targets;
  for (const auto *succ : inst.successors()) {
    targets.push_back(get_block(*succ));
  }
  result["targets"] = std::move(targets);
  return result;
}

json::Object FunctionSerializationContext::serialize_inst_switch(
    const SwitchInst &inst) const {
  json::Object result;
  result["cond_ty"] = serialize_type(*inst.getCondition()->getType());
  result["cond"] = serialize_value(*inst.getCondition());

  const auto &default_case = inst.case_default();
  json::Array targets;
  for (const auto &succ : inst.cases()) {
    if (default_case != inst.case_end() &&
        succ.getCaseIndex() == default_case->getCaseIndex()) {
      continue;
    }
    json::Object item;
    item["block"] = get_block(*succ.getCaseSuccessor());
    item["value"] = serialize_constant(*succ.getCaseValue());
    targets.push_back(std::move(item));
  }
  result["cases"] = std::move(targets);
  if (default_case != inst.case_end()) {
    result["default"] = get_block(*default_case->getCaseSuccessor());
  }
  return result;
}

} // namespace libra