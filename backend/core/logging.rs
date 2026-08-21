//! Process-wide logger setup: colored terminal output plus a rotating-free
//! log file under the app's data directory. Call `init` once, at startup,
//! before anything else logs.

use simplelog::{ColorChoice, CombinedLogger, Config, LevelFilter, TermLogger, TerminalMode, WriteLogger};

use crate::backend::core::errors::{AppError, AppResult};
use crate::configs::app::AppConfig;
use crate::configs::environment::Environment;

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

    CombinedLogger::init(vec![
        TermLogger::new(level, Config::default(), TerminalMode::Mixed, ColorChoice::Auto),
        WriteLogger::new(level, Config::default(), log_file),
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
