#ifndef LIBRA_SHARED_LOGGER_H
#define LIBRA_SHARED_LOGGER_H

#include "Deps.h"

namespace libra {

/// Verbosity flag to control log level
extern cl::opt<bool> OptVerbose;

/// Custom logger for LIBRA passes
class Logger {
public:
  /// The significance or severity of this message.
  enum Level : unsigned char { Debug, Info, Warning, Error, Fatal };

private:
  const Level target_level_;
  const bool no_timestamp_;
  raw_ostream &stm_;

public:
  explicit Logger(Level level, bool no_timestamp)
      : target_level_(level), no_timestamp_(no_timestamp), stm_(llvm::errs()) {}
  ~Logger() = default;

private:
  /// An indicator for the log message level
  static char indicator(Level level) { return "DIWEF"[level]; }

private:
  /// Display one log message to the stream
  void record(Level level, const formatv_object_base &message);

public:
  /// Log a debug message
  template <typename... Ts> void debug(const char *fmt, Ts &&...vals) {
    record(Level::Debug, formatv(fmt, vals...));
  }

  /// Log a info message
  template <typename... Ts> void info(const char *fmt, Ts &&...vals) {
    record(Level::Info, formatv(fmt, vals...));
  }

  /// Log a warning message
  template <typename... Ts> void warning(const char *fmt, Ts &&...vals) {
    record(Level::Warning, formatv(fmt, vals...));
  }

  /// Log a error message
  template <typename... Ts> void error(const char *fmt, Ts &&...vals) {
    record(Level::Error, formatv(fmt, vals...));
  }

  /// Log a fatal message
  template <typename... Ts>
  [[noreturn]] void fatal(const char *fmt, Ts &&...vals) {
    record(Level::Fatal, formatv(fmt, vals...));
    llvm_unreachable("fatal exception happened");
  }
};

/// The global logger
extern std::unique_ptr<Logger> LOG;

/// Create and initialize the default logger
void init_default_logger(Logger::Level level = Logger::Level::Info,
                         bool no_timestamp = false) {
  assert(LOG == nullptr);
  LOG = std::make_unique<Logger>(level, no_timestamp);
}

/// Destroy the default logger and release it
void destroy_default_logger() {
  assert(LOG != nullptr);
  LOG = nullptr;
}

} // namespace libra

#endif // LIBRA_SHARED_LOGGER_H
