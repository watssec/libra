#include "Serializer.h"

namespace {
using namespace libra;

json::Object serialize_const_data_sequence(const ConstantDataSequential &val) {
  json::Object result;
  json::Array elements;
  for (unsigned i = 0; i < val.getNumElements(); i++) {
    elements.push_back(serialize_constant(*val.getElementAsConstant(i)));
  }
  result["elements"] = std::move(elements);
  return result;
}

json::Object serialize_const_pack_aggregate(const ConstantAggregate &val) {
  json::Object result;
  json::Array elements;
  for (unsigned i = 0; i < val.getNumOperands(); i++) {
    elements.push_back(serialize_constant(*val.getOperand(i)));
  }
  result["elements"] = std::move(elements);
  return result;
}

json::Object serialize_const_ref_global(const GlobalValue &val) {
  json::Object result;
  if (val.hasName()) {
    result["name"] = val.getName();
  }
  return result;
}

} // namespace

namespace libra {

json::Object serialize_constant(const Constant &val) {
  json::Object result;
  result["ty"] = serialize_type(*val.getType());
  result["repr"] = serialize_const(val);
  return result;
}

json::Object serialize_const(const Constant &val) {
  json::Object result;

  // markers
  if (isa<DSOLocalEquivalent>(val)) {
    result["Marker"] =
        serialize_const_marker(*cast<DSOLocalEquivalent>(val).getGlobalValue());
  } else if (isa<NoCFIValue>(val)) {
    result["Marker"] =
        serialize_const_marker(*cast<NoCFIValue>(val).getGlobalValue());
  }

  // constant data
  else if (isa<ConstantData>(val)) {
    if (isa<ConstantInt>(val)) {
      result["Int"] = serialize_const_data_int(cast<ConstantInt>(val));
    } else if (isa<ConstantFP>(val)) {
      result["Float"] = serialize_const_data_float(cast<ConstantFP>(val));
    } else if (isa<ConstantPointerNull>(val)) {
      result["Null"] = json::Value(nullptr);
    } else if (isa<ConstantTokenNone>(val)) {
      result["None"] = json::Value(nullptr);
    } else if (isa<ConstantTargetNone>(val)) {
      result["Extension"] = json::Value(nullptr);
    } else if (isa<UndefValue>(val)) {
      result["Undef"] = json::Value(nullptr);
    } else if (isa<ConstantAggregateZero>(val)) {
      result["Default"] = json::Value(nullptr);
    } else if (isa<ConstantDataVector>(val)) {
      result["Vector"] =
          serialize_const_data_vector(cast<ConstantDataVector>(val));
    } else if (isa<ConstantDataArray>(val)) {
      result["Array"] =
          serialize_const_data_array(cast<ConstantDataArray>(val));
    } else {
      LOG->fatal("unknown constant data: {0}", val);
    }
  }

  // constant aggregate
  else if (isa<ConstantAggregate>(val)) {
    if (isa<ConstantVector>(val)) {
      result["Vector"] = serialize_const_pack_vector(cast<ConstantVector>(val));
    } else if (isa<ConstantArray>(val)) {
      result["Array"] = serialize_const_pack_array(cast<ConstantArray>(val));
    } else if (isa<ConstantStruct>(val)) {
      result["Struct"] = serialize_const_pack_struct(cast<ConstantStruct>(val));
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

  // constant block address
  else if (isa<BlockAddress>(val)) {
    result["Label"] = serialize_block_address(cast<BlockAddress>(val));
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
  json::Object result;
  SmallString<1024> dump;
  val.getValue().toStringUnsigned(dump);
  result["value"] = dump;
  return result;
}

json::Object serialize_const_data_float(const ConstantFP &val) {
  json::Object result;
  SmallString<1024> dump;
  val.getValue().toString(dump);
  result["value"] = dump;
  return result;
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

json::Object serialize_const_marker(const GlobalValue &val) {
  json::Object result;
  result["wrap"] = serialize_constant(val);
  return result;
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

json::Object serialize_block_address(const BlockAddress &addr) {
  // sanity checks
  const auto *func = addr.getFunction();
  if (!func->hasName()) {
    LOG->fatal("block address referring to an unnamed function");
  }

  // lookup context
  const auto iter = contexts.find(func);
  if (iter == contexts.cend()) {
    LOG->fatal("function context not ready");
  }
  const auto &ctxt = iter->second;

  // dump the result
  json::Object result;
  result["func"] = func->getName();
  result["block"] = ctxt.get_block(*addr.getBasicBlock());
  return result;
}

json::Object serialize_const_expr(const ConstantExpr &expr) {
  json::Object result;

  FunctionSerializationContext ctxt;
  const auto *inst = expr.getAsInstruction();
  result["inst"] = ctxt.serialize_inst(*inst);

  return result;
}

} // namespace libra