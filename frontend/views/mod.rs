pub mod accounting;
pub mod auth;
pub mod dashboard;
pub mod hr;
pub mod inventory;
pub mod pos;
pub mod reports;
pub mod sales;
pub mod settings;

use std::path::{Path, PathBuf};

use crate::database::database::Database;

/// Shared context every view's `update` needs: the open database, which
/// company's books are being worked on, and where exports get written.
/// Defined here, in the views' shared parent module, rather than
/// inventing a new top-level file for it.
#[derive(Clone)]
pub struct Ctx {
    pub db: Database,
    pub company_id: i64,
    pub current_user_id: Option<i64>,
    pub export_dir: PathBuf,
}

impl Ctx {
    /// Where a report export with the given file name should be written.
    /// Creates the export directory if it doesn't exist yet.
    pub fn export_path(&self, file_name: &str) -> PathBuf {
        let _ = std::fs::create_dir_all(&self.export_dir);
        Path::new(&self.export_dir).join(file_name)
    }
}
