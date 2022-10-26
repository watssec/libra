#include "Serializer.h"

namespace libra {

json::Object serialize_function(const Function &func) {
  json::Object result;

  // basics
  if (func.hasName()) {
    result["name"] = func.getName();
  } else {
    LOG->error("unnamed function: {0}", func);
  }
  result["ty"] = serialize_type(*func.getFunctionType());

  // attributes
  result["is_defined"] = !func.isDeclaration();
  result["is_exact"] = func.isDefinitionExact();
  // TODO: additional attributes or metadata?

  // parameters
  json::Array params;
  for (const auto &param : func.args()) {
    params.push_back(serialize_parameter(param));
  }
  result["params"] = std::move(params);

  // body
  if (func.isIntrinsic()) {
    result["intrinsic"] = func.getIntrinsicID();
  }

  // first label the blocks and instructions
  uint64_t block_counter = 0;
  uint64_t inst_counter = 0;
  std::map<const BasicBlock *, uint64_t> block_labels;
  std::map<const Instruction *, uint64_t> inst_labels;
  for (const auto &block : func) {
    block_labels.emplace(&block, block_counter);
    block_counter++;
    for (const auto &inst : block) {
      inst_labels.emplace(&inst, inst_counter);
      inst_counter++;
    }
  }

  // deserialize the block
  json::Array blocks;
  for (const auto &block : func) {
    blocks.push_back(serialize_block(block, block_labels, inst_labels));
  }
  result["blocks"] = std::move(blocks);

  return result;
}

json::Object serialize_parameter(const Argument &param) {
  json::Object result;

  if (param.hasName()) {
    result["name"] = param.getName();
  }
  result["ty"] = serialize_type(*param.getType());

  return result;
}

json::Object
serialize_block(const BasicBlock &block,
                const std::map<const BasicBlock *, uint64_t> &block_labels,
                const std::map<const Instruction *, uint64_t> &inst_labels) {
  json::Object result;

  // basics
  result["label"] = block_labels.at(&block);
  if (block.hasName()) {
    result["name"] = block.getName();
  }

  // body
  const auto *term = block.getTerminator();

  json::Array body;
  for (const auto &inst : block) {
    // handle terminator separately
    if (term == &inst) {
      continue;
    }
    body.push_back(serialize_instruction(inst, block_labels, inst_labels));
  }
  result["body"] = std::move(body);

  // terminator
  result["terminator"] =
      serialize_instruction(*term, block_labels, inst_labels);

  return result;
}

} // namespace libra