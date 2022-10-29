#include "Deps.h"
#include "Logger.h"
#include "Serializer.h"

using namespace libra;

namespace {

/// Output of the result
cl::opt<std::string> OptOutput("libra-output",
                               cl::desc("The output file name"));

constexpr const char *PASS_NAME = "Libra";

struct LibraPass : PassInfoMixin<LibraPass> {
  // pass entrypoint
  static PreservedAnalyses run(Module &module, ModuleAnalysisManager &) {
    // start of execution
    auto level = Logger::Level::Info;
    if (OptVerbose) {
      level = Logger::Level::Debug;
    }
    init_default_logger(level, OptVerbose);

    // serialize and dump to file
    if (auto e = module.materializeAll()) {
      LOG->fatal("unable to materialize module: {0}", e);
    }

    prepare_for_serialization(module);
    auto data = serialize_module(module);
    std::error_code ec;
    raw_fd_ostream stm(OptOutput, ec,
                       sys::fs::CreationDisposition::CD_CreateNew);
    if (ec) {
      LOG->fatal("unable to create output file: {0}", OptOutput);
    }
    stm << formatv("{0:2}", json::Value(std::move(data)));
    stm.close();

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