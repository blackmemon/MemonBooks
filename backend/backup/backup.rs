//! Creating a backup: `VACUUM INTO` gives a consistent, defragmented
//! snapshot of the live database in one statement, safe to run while the
//! app has the database open (unlike a plain file copy, which could
//! catch a write mid-flight).

use std::path::Path;

use crate::backend::core::errors::{AppError, AppResult};
use crate::database::database::Database;

pub fn create_backup(db: &Database, destination: &Path) -> AppResult<()> {
    if let Some(parent) = destination.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|source| AppError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }
    }

    let destination_str = destination
        .to_str()
        .ok_or_else(|| AppError::Validation("backup destination path is not valid UTF-8".into()))?;

    db.with_connection(|conn| {
        conn.execute("VACUUM INTO ?1", [destination_str])?;
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_a_backup_file_with_the_current_schema() {
        let db = Database::open_in_memory().unwrap();
        db.run_migrations(&[crate::database::database::Migration {
            version: 1,
            name: "widgets",
            sql: "CREATE TABLE widgets (id INTEGER PRIMARY KEY, name TEXT NOT NULL);",
        }])
        .unwrap();
        db.with_connection(|conn| {
            conn.execute("INSERT INTO widgets (name) VALUES ('gizmo')", [])?;
            Ok(())
        })
        .unwrap();

        let dir =
            std::env::temp_dir().join(format!("memonbooks-backup-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let backup_path = dir.join("backup.sqlite3");

        create_backup(&db, &backup_path).unwrap();
        assert!(backup_path.exists());

        let restored = rusqlite::Connection::open(&backup_path).unwrap();
        let count: i64 = restored
            .query_row("SELECT COUNT(*) FROM widgets", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1);

        std::fs::remove_dir_all(&dir).ok();
    }

    /// The full lifecycle this feature exists for: back up a database
    /// with real posted accounting data, restore that backup to a fresh
    /// path via `restore::restore_from_backup` (not just a raw file
    /// copy verified in isolation), open the restored file as its own
    /// `Database`, and confirm every figure -- the company, the
    /// balanced trial balance, the exact posted amount -- matches the
    /// original exactly.
    #[test]
    fn a_backup_restores_into_a_fresh_database_with_all_data_intact() {
        use crate::backend::accounting::accounting_rules::AccountType;
        use crate::backend::accounting::chart_of_accounts::Account;
        use crate::backend::accounting::fiscal_periods::FiscalPeriod;
        use crate::backend::backup::restore;
        use crate::backend::organization::companies::Company;
        use crate::backend::repositories::accounting_repository::{self, AccountingRepository};
        use crate::backend::repositories::organization_repository::{self, OrganizationRepository};
        use crate::backend::transactions::transaction::Transaction;
        use crate::backend::transactions::transaction_lines::TransactionLine;
        use crate::backend::transactions::transaction_types::TransactionType;
        use chrono::NaiveDate;
        use rust_decimal_macros::dec;

        let dir = std::env::temp_dir().join(format!(
            "memonbooks-backup-roundtrip-test-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let original_path = dir.join("original.sqlite3");
        let backup_path = dir.join("backup.sqlite3");
        let restored_path = dir.join("restored.sqlite3");

        // Real domain data: a company, a fiscal period, two accounts, and
        // a posted, balanced transaction.
        let db = Database::open(&original_path).unwrap();
        db.run_migrations(organization_repository::MIGRATIONS)
            .unwrap();
        db.run_migrations(accounting_repository::MIGRATIONS)
            .unwrap();
        db.run_migrations(crate::backend::audit::audit_logs::MIGRATIONS)
            .unwrap();

        let org_repo = OrganizationRepository::new(db.clone());
        let company_id = org_repo
            .create_company(&Company::new("Acme Trading Co", "USD"))
            .unwrap()
            .id
            .unwrap();

        let accounting_repo = AccountingRepository::new(db.clone());
        let period_start = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let period_end = NaiveDate::from_ymd_opt(2026, 1, 31).unwrap();
        accounting_repo
            .create_fiscal_period(&FiscalPeriod::new(
                company_id,
                "January 2026",
                period_start,
                period_end,
            ))
            .unwrap();
        let cash = accounting_repo
            .create_account(&Account::new(
                company_id,
                "1000",
                "Cash",
                AccountType::Asset,
            ))
            .unwrap();
        let capital = accounting_repo
            .create_account(&Account::new(
                company_id,
                "3000",
                "Owner's Capital",
                AccountType::Equity,
            ))
            .unwrap();
        accounting_repo
            .post_transaction(
                &Transaction::new(company_id, TransactionType::OpeningBalance, period_start)
                    .with_line(TransactionLine::debit(cash.id.unwrap(), dec!(7500)))
                    .with_line(TransactionLine::credit(capital.id.unwrap(), dec!(7500))),
            )
            .unwrap();

        create_backup(&db, &backup_path).unwrap();
        drop(db); // release the original connection before restoring elsewhere

        restore::restore_from_backup(&backup_path, &restored_path).unwrap();

        // Open the restored file as an independent database and verify
        // through the same repositories -- not just that bytes landed on
        // disk, but that the data is actually readable and correct.
        let restored_db = Database::open(&restored_path).unwrap();
        let restored_org_repo = OrganizationRepository::new(restored_db.clone());
        let restored_company = restored_org_repo.get_company(company_id).unwrap();
        assert_eq!(restored_company.name, "Acme Trading Co");

        let restored_accounting_repo = AccountingRepository::new(restored_db);
        let trial_balance = restored_accounting_repo
            .trial_balance_as_of(company_id, period_end)
            .unwrap();
        assert!(trial_balance.is_balanced());
        assert_eq!(trial_balance.total_debits(), dec!(7500));

        std::fs::remove_dir_all(&dir).ok();
    }
}
