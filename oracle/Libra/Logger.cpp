#include "Logger.h"

namespace libra {

cl::opt<bool> OptVerbose("libra-verbose", cl::init(false),
                         cl::desc("Verbose logging"));

void Logger::record(Level level, const formatv_object_base &message) {
  if (level < target_level_) {
    return;
  }

  if (no_timestamp_) {
    stm_ << formatv("[{0}] {1}\n", indicator(level), message);
  } else {
    auto timestamp = std::chrono::system_clock::now();
    stm_ << formatv("[{0}] {1:%H:%M:%S.%L} - {2}\n", indicator(level),
                    timestamp, message);
  }
}

std::unique_ptr<Logger> LOG = nullptr;

void init_default_logger(Logger::Level level, bool no_timestamp) {
  assert(LOG == nullptr);
  LOG = std::make_unique<Logger>(level, no_timestamp);
}

void destroy_default_logger() {
  assert(LOG != nullptr);
  LOG = nullptr;
}

} // namespace libra