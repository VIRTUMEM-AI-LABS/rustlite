//! Logging configuration for RustLite
//!
//! Production-grade logging using the `tracing` framework with support
//! for multiple log levels, structured output, and file rotation.

use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Log output destination
#[derive(Debug, Clone)]
pub enum LogOutput {
    /// Output to stdout
    Stdout,
    /// Output to a file with rotation
    File(std::path::PathBuf),
    /// Output to both stdout and file
    Both(std::path::PathBuf),
}

/// Log format style
#[derive(Debug, Clone, Copy)]
pub enum LogFormat {
    /// Human-readable format with colors (default)
    Pretty,
    /// Compact single-line format
    Compact,
}

/// Logging configuration
#[derive(Debug, Clone)]
pub struct LogConfig {
    /// Minimum log level filter
    pub level: String,
    /// Output destination
    pub output: LogOutput,
    /// Format style
    pub format: LogFormat,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            output: LogOutput::Stdout,
            format: LogFormat::Pretty,
        }
    }
}

impl LogConfig {
    /// Create config with info level and stdout output
    pub fn info() -> Self {
        Self {
            level: "info".to_string(),
            ..Default::default()
        }
    }

    /// Create config with debug level
    pub fn debug() -> Self {
        Self {
            level: "debug".to_string(),
            ..Default::default()
        }
    }

    /// Create config with warn level
    pub fn warn() -> Self {
        Self {
            level: "warn".to_string(),
            ..Default::default()
        }
    }

    /// Set log output to file with rotation
    pub fn with_file<P: Into<std::path::PathBuf>>(mut self, path: P) -> Self {
        self.output = LogOutput::File(path.into());
        self
    }

    /// Set log output to both stdout and file
    pub fn with_both<P: Into<std::path::PathBuf>>(mut self, path: P) -> Self {
        self.output = LogOutput::Both(path.into());
        self
    }

    /// Set log format
    pub fn with_format(mut self, format: LogFormat) -> Self {
        self.format = format;
        self
    }

    /// Set log level filter
    pub fn with_level<S: Into<String>>(mut self, level: S) -> Self {
        self.level = level.into();
        self
    }

    /// Initialize global logging with this configuration
    ///
    /// Returns a guard that must be kept alive for logging to work.
    /// When the guard is dropped, the logging worker thread is shutdown.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rustlite::logging::LogConfig;
    ///
    /// // Keep the guard alive for the lifetime of your application
    /// let _guard = LogConfig::info().init();
    /// ```
    pub fn init(self) -> Option<WorkerGuard> {
        let env_filter = EnvFilter::try_from_default_env()
            .or_else(|_| EnvFilter::try_new(&self.level))
            .expect("Invalid log level");

        match self.output {
            LogOutput::Stdout => {
                match self.format {
                    LogFormat::Pretty => {
                        tracing_subscriber::registry()
                            .with(env_filter)
                            .with(fmt::layer().pretty())
                            .init();
                    }
                    LogFormat::Compact => {
                        tracing_subscriber::registry()
                            .with(env_filter)
                            .with(fmt::layer().compact())
                            .init();
                    }
                }
                None
            }
            LogOutput::File(path) => {
                let file_appender = tracing_appender::rolling::daily(
                    path.parent().unwrap_or_else(|| std::path::Path::new(".")),
                    path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("rustlite.log"),
                );
                let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

                match self.format {
                    LogFormat::Pretty => {
                        tracing_subscriber::registry()
                            .with(env_filter)
                            .with(fmt::layer().with_writer(non_blocking).pretty())
                            .init();
                    }
                    LogFormat::Compact => {
                        tracing_subscriber::registry()
                            .with(env_filter)
                            .with(fmt::layer().with_writer(non_blocking).compact())
                            .init();
                    }
                }
                Some(guard)
            }
            LogOutput::Both(path) => {
                let file_appender = tracing_appender::rolling::daily(
                    path.parent().unwrap_or_else(|| std::path::Path::new(".")),
                    path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("rustlite.log"),
                );
                let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

                // Simplified: only use compact format for both outputs to avoid boxing issues
                tracing_subscriber::registry()
                    .with(env_filter)
                    .with(fmt::layer())
                    .with(fmt::layer().with_writer(non_blocking))
                    .init();

                Some(guard)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_config_defaults() {
        let config = LogConfig::default();
        assert_eq!(config.level, "info");
    }

    #[test]
    fn test_log_config_builders() {
        let config = LogConfig::debug()
            .with_file("/tmp/test.log")
            .with_format(LogFormat::Compact);
        assert_eq!(config.level, "debug");
        assert!(matches!(config.output, LogOutput::File(_)));
        assert!(matches!(config.format, LogFormat::Compact));
    }
}
