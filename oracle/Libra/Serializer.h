#ifndef LIBRA_SERIALIZER_H
#define LIBRA_SERIALIZER_H

#include "Config.hpp"
#include "Deps.h"
#include "Logger.h"

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
json::Object serialize_const_ref_global_alias(const GlobalAlias &val);
json::Object serialize_const_ref_global_variable(const GlobalVariable &val);
json::Object serialize_const_ref_function(const Function &val);
json::Object serialize_const_ref_interface(const GlobalIFunc &val);

json::Object serialize_global_variable(const GlobalVariable &gvar);

} // namespace libra

#endif // LIBRA_SERIALIZER_H
