#include "Serializer.h"

namespace {
using namespace libra;

json::Object populate(const Constant &val) {
  json::Object result;
  result["ty"] = serialize_type(*val.getType());
  return result;
}

json::Object serialize_const_data_sequence(const ConstantDataSequential &val) {
  auto result = populate(val);
  json::Array elements;
  for (unsigned i = 0; i < val.getNumElements(); i++) {
    elements.push_back(serialize_const(*val.getElementAsConstant(i)));
  }
  result["elements"] = std::move(elements);
  return result;
}

json::Object serialize_const_pack_aggregate(const ConstantAggregate &val) {
  auto result = populate(val);
  json::Array elements;
  for (unsigned i = 0; i < val.getNumOperands(); i++) {
    elements.push_back(serialize_const(*val.getOperand(i)));
  }
  result["elements"] = std::move(elements);
  return result;
}

json::Object serialize_const_ref_global(const GlobalValue &val) {
  auto result = populate(val);
  if (val.hasName()) {
    result["name"] = val.getName();
  }
  return result;
}

} // namespace

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
    if (isa<GlobalVariable>(val)) {
      result["Variable"] =
          serialize_const_ref_global_variable(cast<GlobalVariable>(val));
    } else if (isa<Function>(val)) {
      result["Function"] = serialize_const_ref_function(cast<Function>(val));
    } else if (isa<GlobalAlias>(val)) {
      result["Alias"] =
          serialize_const_ref_global_alias(cast<GlobalAlias>(val));
    } else if (isa<GlobalIFunc>(val)) {
      result["Interface"] =
          serialize_const_ref_interface(cast<GlobalIFunc>(val));
    } else {
      LOG->fatal("unknown constant reference to global value: {0}", val);
    }
  }

  // constant expression
  else if (isa<ConstantExpr>(val)) {
    result["Expr"] = serialize_const_expr(cast<ConstantExpr>(val));
  }

  // should have exhausted all types of constant
  else {
    LOG->fatal("unknown constant: {0}", val);
  }

  return result;
}

json::Object serialize_const_data_int(const ConstantInt &val) {
  auto result = populate(val);
  if (val.getBitWidth() > OPT_MAX_BITS_FOR_INT) {
    LOG->fatal("constant integer width exceeds limited");
  }
  result["value"] = val.getValue().getLimitedValue(UINT64_MAX);
  return result;
}

json::Object serialize_const_data_float(const ConstantFP &val) {
  auto result = populate(val);
  SmallString<64> dump;
  val.getValue().toString(dump);
  result["value"] = dump;
  return result;
}

json::Object serialize_const_data_ptr_null(const ConstantPointerNull &val) {
  return populate(val);
}

json::Object serialize_const_data_token_none(const ConstantTokenNone &val) {
  return populate(val);
}

json::Object serialize_const_data_undef(const UndefValue &val) {
  return populate(val);
}

json::Object serialize_const_data_all_zero(const ConstantAggregateZero &val) {
  return populate(val);
}

json::Object serialize_const_data_array(const ConstantDataArray &val) {
  return serialize_const_data_sequence(val);
}

json::Object serialize_const_data_vector(const ConstantDataVector &val) {
  return serialize_const_data_sequence(val);
}

json::Object serialize_const_pack_array(const ConstantArray &val) {
  return serialize_const_pack_aggregate(val);
}

json::Object serialize_const_pack_struct(const ConstantStruct &val) {
  return serialize_const_pack_aggregate(val);
}

json::Object serialize_const_pack_vector(const ConstantVector &val) {
  return serialize_const_pack_aggregate(val);
}

json::Object serialize_const_ref_global_variable(const GlobalVariable &val) {
  return serialize_const_ref_global(val);
}

json::Object serialize_const_ref_function(const Function &val) {
  return serialize_const_ref_global(val);
}

json::Object serialize_const_ref_global_alias(const GlobalAlias &val) {
  return serialize_const_ref_global(val);
}

json::Object serialize_const_ref_interface(const GlobalIFunc &val) {
  return serialize_const_ref_global(val);
}

json::Object serialize_const_expr(const ConstantExpr &expr) {
  auto result = populate(expr);
  const auto *inst = expr.getAsInstruction();
  result["repr"] = serialize_inst(*inst);
  return result;
}

} // namespace libra