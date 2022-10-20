#ifndef HISE_SHARED_ORACLE_H
#define HISE_SHARED_ORACLE_H

#include "Deps.h"

namespace hise {

class ModuleOracle {
public:
  // immutable information
  DataLayout data_layout_;

  // common analysis
  CallGraph call_graph_;

  // alias analysis
  GlobalsAAResult alias_result_globals_;

public:
  explicit ModuleOracle(Module &m);

  // explicitly allow move constructor
  ModuleOracle(ModuleOracle &&oracle) noexcept = default;
};

class FunctionOracle {
public:
  // immutable information
  TargetLibraryInfo tl_info_;
  DataLayout data_layout_;

  // mutation caches
  AssumptionCache as_cache_;
  PhiValues phi_vals_;

  // common analysis
  DominatorTree dom_tree_;
  LoopInfo loop_info_;
  ScalarEvolution scev_;

  // alias analysis
  AAResults alias_results;
  BasicAAResult alias_result_basic;
  CFLAndersAAResult alias_result_anserson_;
  CFLSteensAAResult alias_rseult_steens_;
  SCEVAAResult alias_result_scev_;
  TypeBasedAAResult alias_result_type_;
  ScopedNoAliasAAResult alias_result_scope_;

public:
  explicit FunctionOracle(Function &f);

  // explicitly allow move constructor
  FunctionOracle(FunctionOracle &&oracle) noexcept = default;
};

} // namespace hise

#endif // HISE_SHARED_ORACLE_H
