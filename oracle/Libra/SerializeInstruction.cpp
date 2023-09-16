#include "Serializer.h"

// utilities
namespace libra {

std::string get_sync_scope_name(SyncScope::ID scope) {
  switch (scope) {
  case SyncScope::System:
    return "system";
  case SyncScope::SingleThread:
    return "thread";
  default:
    // TODO: handle different SyncScope ID vallues
    return "unknown";
  }
}

} // namespace libra

namespace libra {

json::Object FunctionSerializationContext::serialize_instruction(
    const Instruction &inst) const {
  json::Object result;
  result["ty"] = serialize_type(*inst.getType());
  result["index"] = get_instruction(inst);
  if (inst.hasName()) {
    result["name"] = inst.getName();
  }
  result["repr"] = serialize_inst(inst);
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
  } else if (isa<VAArgInst>(inst)) {
    result["VAArg"] = serialize_inst_va_arg(cast<VAArgInst>(inst));
  }

  // call
  else if (isa<CallInst>(inst)) {
    const auto &call_inst = cast<CallInst>(inst);
    if (isa<IntrinsicInst>(call_inst)) {
      result["Intrinsic"] =
          serialize_inst_call_intrinsic(cast<IntrinsicInst>(call_inst));
    } else if (call_inst.isInlineAsm()) {
      result["CallAsm"] = serialize_inst_call_asm(call_inst);
    } else if (isa<Function>(call_inst.getCalledOperand())) {
      result["CallDirect"] = serialize_inst_call_direct(call_inst);
    } else {
      result["CallIndirect"] = serialize_inst_call_indirect(call_inst);
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
  } else if (isa<FreezeInst>(inst)) {
    result["Freeze"] = serialize_inst_freeze(cast<FreezeInst>(inst));
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

  // aggregates
  else if (isa<ExtractValueInst>(inst)) {
    result["GetValue"] = serialize_inst_get_value(cast<ExtractValueInst>(inst));
  } else if (isa<InsertValueInst>(inst)) {
    result["SetValue"] = serialize_inst_set_value(cast<InsertValueInst>(inst));
  } else if (isa<ExtractElementInst>(inst)) {
    result["GetElement"] =
        serialize_inst_get_element(cast<ExtractElementInst>(inst));
  } else if (isa<InsertElementInst>(inst)) {
    result["SetElement"] =
        serialize_inst_set_element(cast<InsertElementInst>(inst));
  } else if (isa<ShuffleVectorInst>(inst)) {
    result["ShuffleVector"] =
        serialize_inst_shuffle_vector(cast<ShuffleVectorInst>(inst));
  }

  // concurrency instructions
  else if (isa<FenceInst>(inst)) {
    result["Fence"] = serialize_inst_fence(cast<FenceInst>(inst));
  } else if (isa<AtomicCmpXchgInst>(inst)) {
    result["AtomicCmpXchg"] =
        serialize_inst_atomic_cmpxchg(cast<AtomicCmpXchgInst>(inst));
  } else if (isa<AtomicRMWInst>(inst)) {
    result["AtomicRMW"] = serialize_inst_atomic_rmw(cast<AtomicRMWInst>(inst));
  }

  // exception handling (non-terminator)
  else if (isa<LandingPadInst>(inst)) {
    result["LandingPad"] =
        serialize_inst_landing_pad(cast<LandingPadInst>(inst));
  } else if (isa<CatchPadInst>(inst)) {
    // TODO: give details on the CatchPadInst
    result["CatchPad"] = json::Value(nullptr);
  } else if (isa<CleanupPadInst>(inst)) {
    // TODO: give details on the CleanupPadInst
    result["CleanupPad"] = json::Value(nullptr);
  }

  // terminators
  else if (isa<ReturnInst>(inst)) {
    result["Return"] = serialize_inst_return(cast<ReturnInst>(inst));
  } else if (isa<BranchInst>(inst)) {
    result["Branch"] = serialize_inst_branch(cast<BranchInst>(inst));
  } else if (isa<SwitchInst>(inst)) {
    result["Switch"] = serialize_inst_switch(cast<SwitchInst>(inst));
  } else if (isa<IndirectBrInst>(inst)) {
    result["IndirectJump"] =
        serialize_inst_jump_indirect(cast<IndirectBrInst>(inst));
  } else if (isa<InvokeInst>(inst)) {
    const auto &invoke_inst = cast<InvokeInst>(inst);
    if (invoke_inst.isInlineAsm()) {
      result["InvokeAsm"] = serialize_inst_invoke_asm(invoke_inst);
    } else if (isa<Function>(invoke_inst.getCalledOperand())) {
      result["InvokeDirect"] = serialize_inst_invoke_direct(invoke_inst);
    } else {
      result["InvokeIndirect"] = serialize_inst_invoke_indirect(invoke_inst);
    }
  } else if (isa<ResumeInst>(inst)) {
    result["Resume"] = serialize_inst_resume(cast<ResumeInst>(inst));
  } else if (isa<UnreachableInst>(inst)) {
    result["Unreachable"] = json::Value(nullptr);
  }

  // exception handling (terminator)
  else if (isa<CatchSwitchInst>(inst)) {
    // TODO: give details on the CatchSwitchInst
    result["CatchSwitch"] = json::Value(nullptr);
  } else if (isa<CatchReturnInst>(inst)) {
    // TODO: give details on the CatchReturnInst
    result["CatchReturn"] = json::Value(nullptr);
  } else if (isa<CleanupReturnInst>(inst)) {
    // TODO: give details on the CleanupReturnInst
    result["CleanupReturn"] = json::Value(nullptr);
  }

  // very rare cases (terminator)
  else if (isa<CallBrInst>(inst)) {
    // TODO: give details on the CallBrInst
    result["CallBranch"] = json::Value(nullptr);
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
    result["size"] = serialize_value(*inst.getArraySize());
  }
  result["address_space"] = inst.getAddressSpace();
  return result;
}

json::Object
FunctionSerializationContext::serialize_inst_load(const LoadInst &inst) const {
  json::Object result;
  result["pointee_type"] = serialize_type(*inst.getType());
  result["pointer"] = serialize_value(*inst.getPointerOperand());
  result["ordering"] = toIRString(inst.getOrdering());
  result["address_space"] = inst.getPointerAddressSpace();
  return result;
}

json::Object FunctionSerializationContext::serialize_inst_store(
    const StoreInst &inst) const {
  json::Object result;
  result["pointee_type"] = serialize_type(*inst.getValueOperand()->getType());
  result["pointer"] = serialize_value(*inst.getPointerOperand());
  result["value"] = serialize_value(*inst.getValueOperand());
  result["ordering"] = toIRString(inst.getOrdering());
  result["address_space"] = inst.getPointerAddressSpace();
  return result;
}

json::Object FunctionSerializationContext::serialize_inst_va_arg(
    const VAArgInst &inst) const {
  json::Object result;
  result["pointer"] = serialize_value(*inst.getPointerOperand());
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
    result["src_address_space"] =
        cast<PtrToIntInst>(inst).getPointerAddressSpace();
    break;
  }
  case Instruction::CastOps::IntToPtr: {
    result["opcode"] = "int_to_ptr";
    result["dst_address_space"] = cast<IntToPtrInst>(inst).getAddressSpace();
    break;
  }
  case Instruction::CastOps::BitCast: {
    result["opcode"] = "bitcast";
    break;
  }
  case Instruction::CastOps::AddrSpaceCast: {
    result["opcode"] = "address_space_cast";
    result["src_address_space"] =
        cast<AddrSpaceCastInst>(inst).getSrcAddressSpace();
    result["dst_address_space"] =
        cast<AddrSpaceCastInst>(inst).getDestAddressSpace();
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

[[nodiscard]] json::Object FunctionSerializationContext::serialize_inst_freeze(
    const FreezeInst &inst) const {
  json::Object result;
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

json::Object FunctionSerializationContext::serialize_inst_get_value(
    const ExtractValueInst &inst) const {
  json::Object result;
  result["from_ty"] = serialize_type(*inst.getAggregateOperand()->getType());
  result["aggregate"] = serialize_value(*inst.getAggregateOperand());
  json::Array indices;
  for (const auto idx : inst.indices()) {
    indices.push_back(idx);
  }
  result["indices"] = std::move(indices);
  return result;
}

json::Object FunctionSerializationContext::serialize_inst_set_value(
    const InsertValueInst &inst) const {
  json::Object result;
  result["aggregate"] = serialize_value(*inst.getAggregateOperand());
  result["value"] = serialize_value(*inst.getInsertedValueOperand());
  json::Array indices;
  for (const auto idx : inst.indices()) {
    indices.push_back(idx);
  }
  result["indices"] = std::move(indices);
  return result;
}

json::Object FunctionSerializationContext::serialize_inst_get_element(
    const ExtractElementInst &inst) const {
  json::Object result;
  result["vec_ty"] = serialize_type(*inst.getVectorOperandType());
  result["vector"] = serialize_value(*inst.getVectorOperand());
  result["slot"] = serialize_value(*inst.getIndexOperand());
  return result;
}

json::Object FunctionSerializationContext::serialize_inst_set_element(
    const InsertElementInst &inst) const {
  json::Object result;
  result["vector"] = serialize_value(*inst.getOperand(0));
  result["value"] = serialize_value(*inst.getOperand(1));
  result["slot"] = serialize_value(*inst.getOperand(2));
  return result;
}

json::Object FunctionSerializationContext::serialize_inst_shuffle_vector(
    const ShuffleVectorInst &inst) const {
  json::Object result;
  result["lhs"] = serialize_value(*inst.getOperand(0));
  result["rhs"] = serialize_value(*inst.getOperand(1));
  json::Array mask;
  for (const auto val : inst.getShuffleMask()) {
    mask.push_back(val);
  }
  result["mask"] = std::move(mask);
  return result;
}

json::Object FunctionSerializationContext::serialize_inst_fence(
    const FenceInst &inst) const {
  json::Object result;
  result["ordering"] = toIRString(inst.getOrdering());
  result["scope"] = get_sync_scope_name(inst.getSyncScopeID());
  return result;
}

json::Object FunctionSerializationContext::serialize_inst_atomic_cmpxchg(
    const AtomicCmpXchgInst &inst) const {
  json::Object result;

  // basics
  result["pointee_type"] = serialize_type(*inst.getType());
  result["pointer"] = serialize_value(*inst.getPointerOperand());
  result["value_cmp"] = serialize_value(*inst.getCompareOperand());
  result["value_xchg"] = serialize_value(*inst.getNewValOperand());
  result["address_space"] = inst.getPointerAddressSpace();

  // atomicity
  result["ordering_success"] = toIRString(inst.getSuccessOrdering());
  result["ordering_failure"] = toIRString(inst.getFailureOrdering());
  result["scope"] = get_sync_scope_name(inst.getSyncScopeID());

  return result;
}

json::Object FunctionSerializationContext::serialize_inst_atomic_rmw(
    const AtomicRMWInst &inst) const {
  json::Object result;

  // basics
  result["pointee_type"] = serialize_type(*inst.getType());
  result["pointer"] = serialize_value(*inst.getPointerOperand());
  result["value"] = serialize_value(*inst.getValOperand());
  result["address_space"] = inst.getPointerAddressSpace();

  // operand
  switch (inst.getOperation()) {
  case AtomicRMWInst::BinOp::Xchg:
    result["opcode"] = "xchg";
    break;
  case AtomicRMWInst::BinOp::Add:
    result["opcode"] = "add";
    break;
  case AtomicRMWInst::BinOp::FAdd:
    result["opcode"] = "fadd";
    break;
  case AtomicRMWInst::BinOp::Sub:
    result["opcode"] = "sub";
    break;
  case AtomicRMWInst::BinOp::FSub:
    result["opcode"] = "fsub";
    break;
  case AtomicRMWInst::BinOp::UIncWrap:
    result["opcode"] = "uinc";
    break;
  case AtomicRMWInst::BinOp::UDecWrap:
    result["opcode"] = "udec";
    break;
  case AtomicRMWInst::BinOp::Max:
    result["opcode"] = "max";
    break;
  case AtomicRMWInst::BinOp::UMax:
    result["opcode"] = "umax";
    break;
  case AtomicRMWInst::BinOp::FMax:
    result["opcode"] = "fmax";
    break;
  case AtomicRMWInst::BinOp::Min:
    result["opcode"] = "min";
    break;
  case AtomicRMWInst::BinOp::UMin:
    result["opcode"] = "umin";
    break;
  case AtomicRMWInst::BinOp::FMin:
    result["opcode"] = "fmin";
    break;
  case AtomicRMWInst::BinOp::And:
    result["opcode"] = "and";
    break;
  case AtomicRMWInst::BinOp::Or:
    result["opcode"] = "or";
    break;
  case AtomicRMWInst::BinOp::Xor:
    result["opcode"] = "xor";
    break;
  case AtomicRMWInst::BinOp::Nand:
    result["opcode"] = "nand";
    break;
  case AtomicRMWInst::BinOp::BAD_BINOP:
    LOG->fatal("unexpected bad atomic-rmw operator");
  }

  // atomicity
  result["ordering"] = toIRString(inst.getOrdering());
  result["scope"] = get_sync_scope_name(inst.getSyncScopeID());

  return result;
}

json::Object FunctionSerializationContext::serialize_inst_landing_pad(
    const LandingPadInst &inst) const {
  json::Object result;

  json::Array clauses;
  for (unsigned i = 0; i < inst.getNumClauses(); i++) {
    clauses.push_back(serialize_constant(*inst.getClause(i)));
  }
  result["clauses"] = std::move(clauses);
  result["is_cleanup"] = inst.isCleanup();

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

json::Object FunctionSerializationContext::serialize_inst_jump_indirect(
    const IndirectBrInst &inst) const {
  json::Object result;
  result["address"] = serialize_value(*inst.getAddress());
  json::Array targets;
  for (unsigned i = 0; i < inst.getNumDestinations(); i++) {
    targets.push_back(get_block(*inst.getDestination(i)));
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

json::Object FunctionSerializationContext::serialize_inst_invoke_asm(
    const InvokeInst &inst) const {
  json::Object result;
  result["asm"] =
      serialize_inline_asm(*cast<InlineAsm>(inst.getCalledOperand()));

  json::Array args;
  for (const auto &arg : inst.args()) {
    args.push_back(serialize_value(*arg.get()));
  }
  result["args"] = std::move(args);

  result["normal"] = get_block(*inst.getNormalDest());
  result["unwind"] = get_block(*inst.getUnwindDest());
  return result;
}

json::Object FunctionSerializationContext::serialize_inst_invoke_direct(
    const InvokeInst &inst) const {
  json::Object result;
  result["callee"] = serialize_value(*inst.getCalledOperand());
  result["target_type"] = serialize_type(*inst.getFunctionType());

  json::Array args;
  for (const auto &arg : inst.args()) {
    args.push_back(serialize_value(*arg.get()));
  }
  result["args"] = std::move(args);

  result["normal"] = get_block(*inst.getNormalDest());
  result["unwind"] = get_block(*inst.getUnwindDest());
  return result;
}

json::Object FunctionSerializationContext::serialize_inst_invoke_indirect(
    const InvokeInst &inst) const {
  json::Object result;
  result["callee"] = serialize_value(*inst.getCalledOperand());
  result["target_type"] = serialize_type(*inst.getFunctionType());

  json::Array args;
  for (const auto &arg : inst.args()) {
    args.push_back(serialize_value(*arg.get()));
  }
  result["args"] = std::move(args);

  result["normal"] = get_block(*inst.getNormalDest());
  result["unwind"] = get_block(*inst.getUnwindDest());
  return result;
}

json::Object FunctionSerializationContext::serialize_inst_resume(
    const ResumeInst &inst) const {
  json::Object result;
  result["value"] = serialize_value(*inst.getValue());
  return result;
}

} // namespace libra
