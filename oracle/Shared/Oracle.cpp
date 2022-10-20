#include "Oracle.h"

namespace hise {

static TargetLibraryInfo getTargetLibraryInfo(Function &f) {
  TargetLibraryInfoWrapperPass pass;
  return pass.getTLI(f);
}

ModuleOracle::ModuleOracle(Module &m)
    : // immutable information
      data_layout_(&m),
      // common analysis
      call_graph_(m),
      // alias analysis
      alias_result_globals_(GlobalsAAResult::analyzeModule(
          m, getTargetLibraryInfo, call_graph_)) {}

FunctionOracle::FunctionOracle(Function &f)
    : // immutable information
      tl_info_(getTargetLibraryInfo(f)), data_layout_(f.getParent()),
      // mutation caches
      as_cache_(f), phi_vals_(f),
      // common analysis
      dom_tree_(f), loop_info_(dom_tree_),
      scev_(f, tl_info_, as_cache_, dom_tree_, loop_info_),
      // alias analysis
      alias_results(tl_info_),
      alias_result_basic(data_layout_, f, tl_info_, as_cache_, &dom_tree_,
                         &phi_vals_),
      alias_result_anserson_(getTargetLibraryInfo),
      alias_rseult_steens_(getTargetLibraryInfo), alias_result_scev_(scev_),
      alias_result_type_(), alias_result_scope_() {
  // register and combine alias analysis results
  alias_results.addAAResult(alias_result_basic);
  alias_results.addAAResult(alias_result_anserson_);
  alias_results.addAAResult(alias_rseult_steens_);
  alias_results.addAAResult(alias_result_scev_);
  alias_results.addAAResult(alias_result_type_);
  alias_results.addAAResult(alias_result_scope_);
}

} // namespace hise