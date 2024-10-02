#ifndef LIBRA_SERIALIZER_H
#define LIBRA_SERIALIZER_H

#include "Deps.h"
#include "Logger.h"
#include "Metadata.h"

namespace libra {

void prepare_for_serialization(Module &module);

[[nodiscard]] json::Object serialize_module(const Module &module);

[[nodiscard]] json::Object serialize_type(const Type &type);
[[nodiscard]] json::Object serialize_type_int(const IntegerType &type);
[[nodiscard]] json::Object serialize_type_array(const ArrayType &type);
[[nodiscard]] json::Object serialize_type_struct(const StructType &type);
[[nodiscard]] json::Object serialize_type_function(const FunctionType &type);
[[nodiscard]] json::Object serialize_type_pointer(const PointerType &type);
[[nodiscard]] json::Object serialize_type_vector(const VectorType &type);
[[nodiscard]] json::Object serialize_type_extension(const TargetExtType &type);
[[nodiscard]] json::Object
serialize_type_typed_pointer(const TypedPointerType &type);

[[nodiscard]] json::Object serialize_constant(const Constant &val);
[[nodiscard]] json::Object serialize_const(const Constant &val);
[[nodiscard]] json::Object serialize_const_data_int(const ConstantInt &val);
[[nodiscard]] json::Object serialize_const_data_float(const ConstantFP &val);
[[nodiscard]] json::Object
serialize_const_data_array(const ConstantDataArray &val);
[[nodiscard]] json::Object
serialize_const_data_vector(const ConstantDataVector &val);
[[nodiscard]] json::Object serialize_const_pack_array(const ConstantArray &val);
[[nodiscard]] json::Object
serialize_const_pack_struct(const ConstantStruct &val);
[[nodiscard]] json::Object
serialize_const_pack_vector(const ConstantVector &val);
[[nodiscard]] json::Object serialize_const_marker(const GlobalValue &gval);
[[nodiscard]] json::Object
serialize_const_ref_global_variable(const GlobalVariable &val);
[[nodiscard]] json::Object serialize_const_ref_function(const Function &val);
[[nodiscard]] json::Object
serialize_const_ref_global_alias(const GlobalAlias &val);
[[nodiscard]] json::Object
serialize_const_ref_interface(const GlobalIFunc &val);
[[nodiscard]] json::Object serialize_block_address(const BlockAddress &addr);
[[nodiscard]] json::Object serialize_const_expr(const ConstantExpr &expr);

[[nodiscard]] json::Object
serialize_global_variable(const GlobalVariable &gvar);

[[nodiscard]] json::Object serialize_function(const Function &func);
[[nodiscard]] json::Object serialize_parameter(const Argument &param);

[[nodiscard]] json::Object serialize_inline_asm(const InlineAsm &assembly);

class FunctionSerializationContext {
private:
  std::map<const BasicBlock *, uint64_t> block_labels_;
  std::map<const Instruction *, uint64_t> inst_labels_;
  std::map<const Argument *, uint64_t> arg_labels_;

public:
  FunctionSerializationContext() = default;

public:
  void add_block(const BasicBlock &block);
  void add_instruction(const Instruction &inst);
  void add_argument(const Argument &arg);

public:
  [[nodiscard]] uint64_t get_block(const BasicBlock &block) const;
  [[nodiscard]] uint64_t get_instruction(const Instruction &inst) const;
  [[nodiscard]] uint64_t get_argument(const Argument &arg) const;

public:
  [[nodiscard]] json::Object serialize_block(const BasicBlock &block) const;

  [[nodiscard]] json::Object
  serialize_instruction(const Instruction &inst) const;
  [[nodiscard]] json::Object serialize_inst(const Instruction &inst) const;
  [[nodiscard]] json::Object
  serialize_inst_alloca(const AllocaInst &inst) const;
  [[nodiscard]] json::Object serialize_inst_load(const LoadInst &inst) const;
  [[nodiscard]] json::Object serialize_inst_store(const StoreInst &inst) const;
  [[nodiscard]] json::Object serialize_inst_va_arg(const VAArgInst &inst) const;
  [[nodiscard]] json::Object
  serialize_inst_call_asm(const CallInst &inst) const;
  [[nodiscard]] json::Object
  serialize_inst_call_direct(const CallInst &inst) const;
  [[nodiscard]] json::Object
  serialize_inst_call_indirect(const CallInst &inst) const;
  [[nodiscard]] json::Object
  serialize_inst_call_intrinsic(const IntrinsicInst &inst) const;
  [[nodiscard]] json::Object
  serialize_inst_unary_operator(const UnaryOperator &inst) const;
  [[nodiscard]] json::Object
  serialize_inst_binary_operator(const BinaryOperator &inst) const;
  [[nodiscard]] json::Object serialize_inst_compare(const CmpInst &inst) const;
  [[nodiscard]] json::Object serialize_inst_cast(const CastInst &inst) const;
  [[nodiscard]] json::Object
  serialize_inst_freeze(const FreezeInst &inst) const;
  [[nodiscard]] json::Object
  serialize_inst_gep(const GetElementPtrInst &inst) const;
  [[nodiscard]] json::Object serialize_inst_phi(const PHINode &inst) const;
  [[nodiscard]] json::Object serialize_inst_ite(const SelectInst &inst) const;
  [[nodiscard]] json::Object
  serialize_inst_get_value(const ExtractValueInst &inst) const;
  [[nodiscard]] json::Object
  serialize_inst_set_value(const InsertValueInst &inst) const;
  [[nodiscard]] json::Object
  serialize_inst_get_element(const ExtractElementInst &inst) const;
  [[nodiscard]] json::Object
  serialize_inst_set_element(const InsertElementInst &inst) const;
  [[nodiscard]] json::Object
  serialize_inst_shuffle_vector(const ShuffleVectorInst &inst) const;
  [[nodiscard]] json::Object serialize_inst_fence(const FenceInst &inst) const;
  [[nodiscard]] json::Object
  serialize_inst_atomic_cmpxchg(const AtomicCmpXchgInst &inst) const;
  [[nodiscard]] json::Object
  serialize_inst_atomic_rmw(const AtomicRMWInst &inst) const;
  [[nodiscard]] json::Object
  serialize_inst_landing_pad(const LandingPadInst &inst) const;
  [[nodiscard]] json::Object
  serialize_inst_return(const ReturnInst &inst) const;
  [[nodiscard]] json::Object
  serialize_inst_branch(const BranchInst &inst) const;
  [[nodiscard]] json::Object
  serialize_inst_jump_indirect(const IndirectBrInst &inst) const;
  [[nodiscard]] json::Object
  serialize_inst_switch(const SwitchInst &inst) const;
  [[nodiscard]] json::Object
  serialize_inst_invoke_asm(const InvokeInst &inst) const;
  [[nodiscard]] json::Object
  serialize_inst_invoke_direct(const InvokeInst &inst) const;
  [[nodiscard]] json::Object
  serialize_inst_invoke_indirect(const InvokeInst &inst) const;
  [[nodiscard]] json::Object
  serialize_inst_resume(const ResumeInst &inst) const;

  [[nodiscard]] json::Object serialize_value(const Value &val) const;
  [[nodiscard]] json::Object
  serialize_value_argument(const Argument &arg) const;
  [[nodiscard]] json::Object
  serialize_value_block(const BasicBlock &block) const;
  [[nodiscard]] json::Object
  serialize_value_instruction(const Instruction &inst) const;
};

// TODO: use a more elegant design
// module-level context
extern std::map<const Function *, FunctionSerializationContext> contexts;

} // namespace libra

#endif // LIBRA_SERIALIZER_H
