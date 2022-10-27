#include "Serializer.h"

namespace libra {

json::Object serialize_inline_asm(const InlineAsm &assembly) {
  json::Object result;
  result["asm"] = assembly.getAsmString();
  result["constraint"] = assembly.getConstraintString();
  return result;
}

} // namespace libra