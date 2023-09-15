#include "Serializer.h"

namespace {

json::Object mk_float(uint64_t width, const char *name) {
  json::Object result;
  result["width"] = width;
  result["name"] = name;
  return result;
}

} // namespace

namespace libra {

json::Object serialize_type(const Type &type) {
  json::Object result;

  switch (type.getTypeID()) {
  case Type::TypeID::VoidTyID:
    result["Void"] = json::Value(nullptr);
    break;
  case Type::TypeID::IntegerTyID:
    result["Int"] = serialize_type_int(cast<IntegerType>(type));
    break;
  case Type::TypeID::HalfTyID:
    result["Float"] = mk_float(16, "half");
    break;
  case Type::TypeID::BFloatTyID:
    result["Float"] = mk_float(16, "bfloat");
    break;
  case Type::TypeID::FloatTyID:
    result["Float"] = mk_float(32, "float");
    break;
  case Type::TypeID::DoubleTyID:
    result["Float"] = mk_float(64, "double");
    break;
  case Type::TypeID::X86_FP80TyID:
    result["Float"] = mk_float(80, "x86_fp80");
    break;
  case Type::TypeID::FP128TyID:
    result["Float"] = mk_float(128, "fp128");
    break;
  case Type::TypeID::PPC_FP128TyID:
    result["Float"] = mk_float(128, "ppc_fp128");
    break;
  case Type::TypeID::ArrayTyID:
    result["Array"] = serialize_type_array(cast<ArrayType>(type));
    break;
  case Type::TypeID::StructTyID:
    result["Struct"] = serialize_type_struct(cast<StructType>(type));
    break;
  case Type::TypeID::FunctionTyID:
    result["Function"] = serialize_type_function(cast<FunctionType>(type));
    break;
  case Type::TypeID::PointerTyID:
    result["Pointer"] = serialize_type_pointer(cast<PointerType>(type));
    break;
  case Type::FixedVectorTyID:
  case Type::ScalableVectorTyID:
    result["Vector"] = serialize_type_vector(cast<VectorType>(type));
    break;
  case Type::TargetExtTyID:
    result["Extension"] = serialize_type_extension(cast<TargetExtType>(type));
    break;
  case Type::TypedPointerTyID:
    result["TypedPointer"] =
        serialize_type_typed_pointer(cast<TypedPointerType>(type));
    break;
  case Type::LabelTyID:
    result["Label"] = json::Value(nullptr);
    break;
  case Type::TokenTyID:
    // TODO: it is arguable whether X86_* types should be token
  case Type::X86_AMXTyID:
  case Type::X86_MMXTyID:
    result["Token"] = json::Value(nullptr);
    break;
  case Type::MetadataTyID:
    result["Metadata"] = json::Value(nullptr);
    break;
  }
  return result;
}

json::Object serialize_type_int(const IntegerType &type) {
  json::Object result;
  result["width"] = type.getBitWidth();
  return result;
}

json::Object serialize_type_array(const ArrayType &type) {
  json::Object result;
  result["element"] = serialize_type(*type.getElementType());
  result["length"] = type.getNumElements();
  return result;
}

json::Object serialize_type_struct(const StructType &type) {
  json::Object result;

  if (type.hasName()) {
    result["name"] = type.getName();
  }

  // collect fields only when non-opaque
  if (!type.isOpaque()) {
    json::Array fields;
    for (const auto *field : type.elements()) {
      fields.push_back(serialize_type(*field));
    }
    result["fields"] = std::move(fields);
  }

  return result;
}

json::Object serialize_type_function(const FunctionType &type) {
  json::Object result;

  json::Array params;
  for (const auto *param : type.params()) {
    params.push_back(serialize_type(*param));
  }
  result["params"] = std::move(params);
  result["variadic"] = type.isVarArg();
  result["ret"] = serialize_type(*type.getReturnType());

  return result;
}

json::Object serialize_type_pointer(const PointerType &type) {
  json::Object result;
  result["address_space"] = type.getAddressSpace();
  return result;
}

json::Object serialize_type_vector(const VectorType &type) {
  json::Object result;

  result["element"] = serialize_type(*type.getElementType());
  if (isa<FixedVectorType>(type)) {
    result["fixed"] = true;
    result["length"] = cast<FixedVectorType>(type).getNumElements();
  } else if (isa<ScalableVectorType>(type)) {
    result["fixed"] = false;
    result["length"] = cast<ScalableVectorType>(type).getMinNumElements();
  } else {
    llvm_unreachable("invalid vector type");
  }

  return result;
}

json::Object serialize_type_extension(const TargetExtType &type) {
  json::Object result;

  result["name"] = type.getName();

  json::Array params;
  for (const auto *param : type.type_params()) {
    params.push_back(serialize_type(*param));
  }
  result["params"] = std::move(params);

  return result;
}

json::Object serialize_type_typed_pointer(const TypedPointerType &type) {
  json::Object result;
  result["pointee"] = serialize_type(*type.getElementType());
  result["address_space"] = type.getAddressSpace();
  return result;
}

} // namespace libra