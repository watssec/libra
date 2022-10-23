#ifndef LIBRA_SERIALIZER_H
#define LIBRA_SERIALIZER_H

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

json::Object serialize_global_variable(const GlobalVariable &gvar);

} // namespace libra

#endif // LIBRA_SERIALIZER_H
