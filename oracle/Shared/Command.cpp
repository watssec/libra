#include "Command.h"

namespace libra {

cl::opt<bool> OptTest("libra-test", cl::init(false),
                      cl::desc("Run the pass in testing mode"));
cl::opt<bool> OptVerbose("libra-verbose", cl::init(false),
                         cl::desc("Verbose logging"));

} // namespace libra