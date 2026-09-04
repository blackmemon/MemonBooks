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
    /// Where a view-attached file (e.g. Chart of Accounts' attachments)
    /// is copied to once picked -- a runtime data directory, not the
    /// `frontend/uploads/` scaffold in the source tree, since real
    /// user files can't live inside the packaged app itself.
    pub uploads_dir: PathBuf,
}

impl Ctx {
    /// Where a report export with the given file name should be written.
    /// Creates the export directory if it doesn't exist yet.
    pub fn export_path(&self, file_name: &str) -> PathBuf {
        let _ = std::fs::create_dir_all(&self.export_dir);
        Path::new(&self.export_dir).join(file_name)
    }

    /// Where an attached file with the given (already collision-safe)
    /// file name should be copied to. Creates the uploads directory if
    /// it doesn't exist yet.
    pub fn upload_path(&self, file_name: &str) -> PathBuf {
        let _ = std::fs::create_dir_all(&self.uploads_dir);
        Path::new(&self.uploads_dir).join(file_name)
    }
}
