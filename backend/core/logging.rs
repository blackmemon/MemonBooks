//! Process-wide logger setup: colored terminal output plus a rotating-free
//! log file under the app's data directory. Call `init` once, at startup,
//! before anything else logs.

use simplelog::{ColorChoice, CombinedLogger, ConfigBuilder, LevelFilter, TermLogger, TerminalMode, WriteLogger};

use crate::backend::core::errors::{AppError, AppResult};
use crate::configs::app::AppConfig;
use crate::configs::environment::Environment;

/// Third-party crates that log through the same global `log` facade we
/// do. Their own DEBUG output (GPU adapter enumeration, shader
/// compilation, window-system chatter) is noisy enough to grow the log
/// file into the megabytes within seconds -- ignored here so DEBUG level
/// stays useful for our own code.
const NOISY_DEBUG_TARGETS: &[&str] = &["wgpu", "naga", "iced_wgpu", "iced_winit", "iced_graphics", "winit", "cosmic_text"];

fn level_for(environment: Environment) -> LevelFilter {
    match environment {
        Environment::Development | Environment::Test => LevelFilter::Debug,
        Environment::Production => LevelFilter::Info,
    }
}

pub fn init(config: &AppConfig) -> AppResult<()> {
    let level = level_for(config.environment);

    if let Some(parent) = config.log_path.parent() {
        std::fs::create_dir_all(parent).map_err(|source| AppError::Io {
            path: parent.to_path_buf(),
            source,
        })?;
    }

    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&config.log_path)
        .map_err(|source| AppError::Io {
            path: config.log_path.clone(),
            source,
        })?;

    let mut builder = ConfigBuilder::new();
    if level >= LevelFilter::Debug {
        for target in NOISY_DEBUG_TARGETS {
            builder.add_filter_ignore_str(target);
        }
    }
    let log_config = builder.build();

    CombinedLogger::init(vec![
        TermLogger::new(level, log_config.clone(), TerminalMode::Mixed, ColorChoice::Auto),
        WriteLogger::new(level, log_config, log_file),
    ])
    .map_err(|err| AppError::Config(format!("failed to initialize logging: {err}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn production_logs_at_info_development_and_test_log_at_debug() {
        assert_eq!(level_for(Environment::Production), LevelFilter::Info);
        assert_eq!(level_for(Environment::Development), LevelFilter::Debug);
        assert_eq!(level_for(Environment::Test), LevelFilter::Debug);
    }
}
