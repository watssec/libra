#include "Serializer.h"

namespace {
using namespace libra;

json::Object
populate(const Instruction &inst,
         const std::map<const Instruction *, uint64_t> &inst_labels) {
  json::Object result;
  result["ty"] = serialize_type(*inst.getType());
  result["index"] = inst_labels.at(&inst);
  return result;
}

} // namespace

namespace libra {

json::Object serialize_instruction(
    const Instruction &inst,
    const std::map<const BasicBlock *, uint64_t> &block_labels,
    const std::map<const Instruction *, uint64_t> &inst_labels) {}

} // namespace libra