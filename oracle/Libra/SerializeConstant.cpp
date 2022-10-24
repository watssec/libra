#include "Serializer.h"

namespace libra {

json::Object serialize_const(const Constant &val) {
  json::Object result;

  // early filtering
  if (isa<BlockAddress>(val)) {
    LOG->fatal("serializing a block address as constant");
  } else if (isa<DSOLocalEquivalent>(val)) {
    LOG->fatal("serializing a dso_local marker");
  } else if (isa<NoCFIValue>(val)) {
    LOG->fatal("serializing a no-CFI marker");
  }

  // constant data
  else if (isa<ConstantData>(val)) {
    if (isa<ConstantInt>(val)) {
      result["Int"] = serialize_const_data_int(cast<ConstantInt>(val));
    } else if (isa<ConstantFP>(val)) {
      result["Float"] = serialize_const_data_float(cast<ConstantFP>(val));
    } else if (isa<ConstantPointerNull>(val)) {
      result["Null"] =
          serialize_const_data_ptr_null(cast<ConstantPointerNull>(val));
    } else if (isa<ConstantTokenNone>(val)) {
      result["None"] =
          serialize_const_data_token_none(cast<ConstantTokenNone>(val));
    } else if (isa<UndefValue>(val)) {
      result["Undef"] = serialize_const_data_undef(cast<UndefValue>(val));
    } else if (isa<ConstantAggregateZero>(val)) {
      result["Default"] =
          serialize_const_data_all_zero(cast<ConstantAggregateZero>(val));
    } else if (isa<ConstantDataArray>(val)) {
      result["Array"] =
          serialize_const_data_array(cast<ConstantDataArray>(val));
    } else if (isa<ConstantDataVector>(val)) {
      result["Vector"] =
          serialize_const_data_vector(cast<ConstantDataVector>(val));
    } else {
      LOG->fatal("unknown constant data: {0}", val);
    }
  }

  // constant aggregate
  else if (isa<ConstantAggregate>(val)) {
    if (isa<ConstantArray>(val)) {
      result["Array"] = serialize_const_pack_array(cast<ConstantArray>(val));
    } else if (isa<ConstantStruct>(val)) {
      result["Struct"] = serialize_const_pack_struct(cast<ConstantStruct>(val));
    } else if (isa<ConstantVector>(val)) {
      result["Vector"] = serialize_const_pack_vector(cast<ConstantVector>(val));
    } else {
      LOG->fatal("unknown constant aggregate: {0}", val);
    }
  }

  // reference to global declarations
  else if (isa<GlobalValue>(val)) {
    if (isa<GlobalAlias>(val)) {
      result["Alias"] =
          serialize_const_ref_global_alias(cast<GlobalAlias>(val));
    } else if (isa<GlobalVariable>(val)) {
      result["Variable"] =
          serialize_const_ref_global_variable(cast<GlobalVariable>(val));
    } else if (isa<Function>(val)) {
      result["Function"] = serialize_const_ref_function(cast<Function>(val));
    } else if (isa<GlobalIFunc>(val)) {
      result["Interface"] =
          serialize_const_ref_interface(cast<GlobalIFunc>(val));
    } else {
      LOG->fatal("unknown constant reference to global value: {0}", val);
    }
  }

  // TODO: constant expression

  // should have exhausted all types of constant
  else {
    LOG->fatal("unknown constant: {0}", val);
  }

  return result;
}

} // namespace libra