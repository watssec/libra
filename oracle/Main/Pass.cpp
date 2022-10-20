#include "Shared/Command.h"
#include "Shared/Deps.h"
#include "Shared/Logger.h"

using namespace hise;

namespace {

constexpr const char *PASS_NAME = "HiseSymbolizer";

struct HiseSymbolizer : PassInfoMixin<HiseSymbolizer> {
  // pass entrypoint
  static PreservedAnalyses run(Module &module, ModuleAnalysisManager &) {
    // start of execution
    auto level = Logger::Level::Info;
    if (OptTest || OptVerbose) {
      level = Logger::Level::Debug;
    }
    init_default_logger(level, OptTest);

    if (OptTest) {
      LOG->debug("==== testing ====");
    }

    // end of execution
    if (OptTest) {
      LOG->debug("==== test ok ====");
    }
    destroy_default_logger();

    // mark that all analysis results are invalidated
    return PreservedAnalyses::none();
  }

  // force every module to go through this pass.
  static bool isRequired() { return true; }
};

//-----------------------------------------------------------------------------
// Pass Registration
//-----------------------------------------------------------------------------
llvm::PassPluginLibraryInfo getPassInfo() {
  return {LLVM_PLUGIN_API_VERSION, PASS_NAME, LLVM_VERSION_STRING,
          [](PassBuilder &PB) {
            // allow this pass to run directly by name
            PB.registerPipelineParsingCallback(
                [](StringRef Name, ModulePassManager &MPM,
                   ArrayRef<PassBuilder::PipelineElement>) {
                  if (Name == PASS_NAME) {
                    MPM.addPass(HiseSymbolizer());
                    return true;
                  }
                  return false;
                });
          }};
}

/// Register the pass to the `opt` interface
extern "C" LLVM_ATTRIBUTE_WEAK ::llvm::PassPluginLibraryInfo
llvmGetPassPluginInfo() {
  return getPassInfo();
}

} // namespace