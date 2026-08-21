//! Resolved application configuration: which environment we're running in
//! and where on disk this platform stores the app's database and logs.

use std::path::PathBuf;

use directories::ProjectDirs;

use crate::backend::core::errors::{AppError, AppResult};
use crate::configs::environment::Environment;

const QUALIFIER: &str = "com";
const ORGANIZATION: &str = "MemonBooks";
const APPLICATION: &str = "MemonBooks";

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub environment: Environment,
    pub data_dir: PathBuf,
    pub database_path: PathBuf,
    pub log_path: PathBuf,
}

impl AppConfig {
    /// Resolves the platform-appropriate app data directory (via the
    /// `directories` crate: e.g. `%APPDATA%\MemonBooks\MemonBooks` on
    /// Windows, `~/Library/Application Support/com.MemonBooks.MemonBooks`
    /// on macOS, `~/.local/share/memonbooks` on Linux) and ensures it
    /// exists on disk.
    pub fn load() -> AppResult<Self> {
        let project_dirs = ProjectDirs::from(QUALIFIER, ORGANIZATION, APPLICATION).ok_or_else(|| {
            AppError::Config("could not determine an application data directory for this platform".into())
        })?;

        let data_dir = project_dirs.data_dir().to_path_buf();
        std::fs::create_dir_all(&data_dir).map_err(|source| AppError::Io {
            path: data_dir.clone(),
            source,
        })?;

        Ok(Self::from_data_dir(data_dir, Environment::current()))
    }

    /// Pure constructor: derives the database/log paths from a given data
    /// directory without touching the filesystem or the environment, so
    /// it can be unit tested directly.
    fn from_data_dir(data_dir: PathBuf, environment: Environment) -> Self {
        let database_path = data_dir.join("memonbooks.sqlite3");
        let log_path = data_dir.join("logs").join("memonbooks.log");

        Self {
            environment,
            data_dir,
            database_path,
            log_path,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derives_database_and_log_paths_under_data_dir() {
        let config = AppConfig::from_data_dir(PathBuf::from("/tmp/memonbooks-test"), Environment::Test);

        assert_eq!(config.database_path, PathBuf::from("/tmp/memonbooks-test/memonbooks.sqlite3"));
        assert_eq!(config.log_path, PathBuf::from("/tmp/memonbooks-test/logs/memonbooks.log"));
        assert_eq!(config.environment, Environment::Test);
    }
}
