#include "Serializer.h"

namespace libra {

json::Object serialize_type(const Type &type) {
  json::Object result;

  switch (type.getTypeID()) {
  case Type::TypeID::VoidTyID:
    result["Void"] = {};
    break;
  case Type::TypeID::IntegerTyID:
    result["Int"] = serialize_type_int(cast<IntegerType>(type));
    break;
  case Type::TypeID::HalfTyID:
    result["Float"] = {
        {"width", 16},
        {"name", "half"},
    };
    break;
  case Type::TypeID::BFloatTyID:
    result["Float"] = {
        {"width", 16},
        {"name", "bfloat"},
    };
    break;
  case Type::TypeID::FloatTyID:
    result["Float"] = {
        {"width", 32},
        {"name", "float"},
    };
    break;
  case Type::TypeID::DoubleTyID:
    result["Float"] = {
        {"width", 64},
        {"name", "double"},
    };
    break;
  case Type::TypeID::X86_FP80TyID:
    result["Float"] = {
        {"width", 80},
        {"name", "x86_fp80"},
    };
    break;
  case Type::TypeID::FP128TyID:
    result["Float"] = {
        {"width", 128},
        {"name", "fp128"},
    };
    break;
  case Type::TypeID::PPC_FP128TyID:
    result["Float"] = {
        {"width", 128},
        {"name", "ppc_fp128"},
    };
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
  case Type::X86_AMXTyID:
  case Type::X86_MMXTyID:
    result["Vector"] = serialize_type_vector(cast<VectorType>(type));
    break;
  case Type::LabelTyID:
    result["Label"] = {};
    break;
  case Type::TokenTyID:
    result["Token"] = {};
    break;
  case Type::MetadataTyID:
    result["Metadata"] = {};
    break;
  case Type::DXILPointerTyID:
    result["Other"] = {{"name", "DXIL pointer"}};
    break;
  }
  return result;
}

json::Object serialize_type_int(const IntegerType &type) {
  json::Object result;
  result["width"] = type.getBitWidth();
  result["mask"] = type.getBitMask();
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
  if (!type.isOpaque()) {
    result["pointee"] = serialize_type(*type.getNonOpaquePointerElementType());
  }
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

} // namespace libra