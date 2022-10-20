#include "Logger.h"

namespace libra {

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

} // namespace libra