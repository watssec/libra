#include "Deps.h"
#include "Logger.h"

using namespace libra;

namespace {

constexpr const char *PASS_NAME = "LIBRA";

struct LibraPass : PassInfoMixin<LibraPass> {
  // pass entrypoint
  static PreservedAnalyses run(Module &module, ModuleAnalysisManager &) {
    // start of execution
    auto level = Logger::Level::Info;
    if (OptVerbose) {
      level = Logger::Level::Debug;
    }
    init_default_logger(level, OptVerbose);

    // end of execution
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
                    MPM.addPass(LibraPass());
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