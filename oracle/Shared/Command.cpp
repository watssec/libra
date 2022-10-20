#include "Command.h"

namespace hise {

cl::opt<bool> OptTest("hise-test", cl::init(false),
                      cl::desc("Run the pass in testing mode"));
cl::opt<bool> OptVerbose("hise-verbose", cl::init(false),
                         cl::desc("Verbose logging"));

} // namespace hise