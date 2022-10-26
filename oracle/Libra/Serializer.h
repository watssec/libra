#ifndef LIBRA_SERIALIZER_H
#define LIBRA_SERIALIZER_H

#include "Config.hpp"
#include "Deps.h"
#include "Logger.h"
#include "Metadata.h"

namespace libra {

json::Object serialize_module(const Module &module);

json::Object serialize_type(const Type &type);
json::Object serialize_type_int(const IntegerType &type);
json::Object serialize_type_array(const ArrayType &type);
json::Object serialize_type_struct(const StructType &type);
json::Object serialize_type_function(const FunctionType &type);
json::Object serialize_type_pointer(const PointerType &type);
json::Object serialize_type_vector(const VectorType &type);

json::Object serialize_const(const Constant &val);
json::Object serialize_const_data_int(const ConstantInt &val);
json::Object serialize_const_data_float(const ConstantFP &val);
json::Object serialize_const_data_ptr_null(const ConstantPointerNull &val);
json::Object serialize_const_data_token_none(const ConstantTokenNone &val);
json::Object serialize_const_data_undef(const UndefValue &val);
json::Object serialize_const_data_all_zero(const ConstantAggregateZero &val);
json::Object serialize_const_data_array(const ConstantDataArray &val);
json::Object serialize_const_data_vector(const ConstantDataVector &val);
json::Object serialize_const_pack_array(const ConstantArray &val);
json::Object serialize_const_pack_struct(const ConstantStruct &val);
json::Object serialize_const_pack_vector(const ConstantVector &val);
json::Object serialize_const_ref_global_variable(const GlobalVariable &val);
json::Object serialize_const_ref_function(const Function &val);
json::Object serialize_const_ref_global_alias(const GlobalAlias &val);
json::Object serialize_const_ref_interface(const GlobalIFunc &val);
json::Object serialize_const_expr(const ConstantExpr &expr);

json::Object serialize_global_variable(const GlobalVariable &gvar);

json::Object serialize_function(const Function &func);
json::Object serialize_parameter(const Argument &param);

json::Object serialize_inline_asm(const InlineAsm &assembly);

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

private:
  uint64_t get_block(const BasicBlock &block) const;
  uint64_t get_instruction(const Instruction &inst) const;
  uint64_t get_argument(const Argument &arg) const;

public:
  json::Object serialize_block(const BasicBlock &block) const;

  json::Object serialize_instruction(const Instruction &inst) const;
  json::Object serialize_inst(const Instruction &inst) const;
  json::Object serialize_inst_alloca(const AllocaInst &inst) const;

  json::Object serialize_value(const Value &val) const;
  json::Object serialize_value_argument(const Argument &arg) const;
  json::Object serialize_value_instruction(const Instruction &inst) const;
};

} // namespace libra

#endif // LIBRA_SERIALIZER_H
