//! A company's chart of accounts: the set of ledger accounts everything
//! else (journal entries, the general ledger, financial reports) posts
//! against or derives from.

use crate::backend::accounting::accounting_rules::{AccountType, NormalBalance};
use crate::backend::core::errors::{AppError, AppResult};
use crate::backend::core::reference_code;
use crate::backend::core::validation::{require_max_length, require_non_empty, Validate};

/// `Account::status` value meaning the account can be posted to and
/// shows up in pickers -- the sole source of truth `Account::is_active`
/// checks against. Chart of Accounts-specific fields (Batch: adding
/// `account_code`/`account_name`/`parent_account`/`status`/
/// `description`/`attachments`/`create_timestamp`/`modified_timestamp`)
/// live here on the shared ledger model rather than the view, since
/// every other subsystem that posts against an account (journal
/// entries, reports, banking, POS, ...) needs the real field, not a
/// UI-local copy.
pub const STATUS_ACTIVE: &str = "Active";
/// The complementary `Account::status` value -- excluded from pickers
/// and from `StatusFilter::Active`, but never deleted outright.
pub const STATUS_INACTIVE: &str = "Inactive";

#[derive(Debug, Clone, PartialEq)]
pub struct Account {
    pub id: Option<i64>,
    pub company_id: i64,
    /// Account number, e.g. "1000". Unique within a company. This is
    /// the short, human-assigned number that drives sorting, the "1000-
    /// 1999 Current Assets"-style hierarchy, and every account picker
    /// label in the app -- it stays exactly as-is; `reference_code`
    /// below is a separate, auto-generated internal ID, not a
    /// replacement for it.
    pub account_code: String,
    pub account_name: String,
    pub account_type: AccountType,
    /// Parent account, for a hierarchical chart of accounts (e.g. "1000
    /// Cash" rolling up into a "1000-1999 Current Assets" heading).
    pub parent_account: Option<i64>,
    /// [`STATUS_ACTIVE`] or [`STATUS_INACTIVE`] -- see [`Account::is_active`].
    pub status: String,
    /// Auto-generated, collision-safe internal reference id (`COA-
    /// YYYYMMDDHHMMSSmmm`) -- see `core::reference_code`. `None` only
    /// for accounts created before this field existed; every account
    /// created through `Account::new` gets one automatically.
    pub reference_code: Option<String>,
    pub description: String,
    /// File names of documents attached to this account, stored under
    /// `Ctx::upload_path` -- the account row itself only keeps the
    /// names, not the file bytes.
    pub attachments: Vec<String>,
    /// Unix timestamp (seconds) set once at creation, never touched
    /// again -- see [`Account::modified_timestamp`] for the one that
    /// does change.
    pub create_timestamp: i64,
    /// Unix timestamp (seconds) refreshed on every successful update,
    /// starting out equal to `create_timestamp`.
    pub modified_timestamp: i64,
}

impl Account {
    pub fn new(
        company_id: i64,
        account_code: impl Into<String>,
        account_name: impl Into<String>,
        account_type: AccountType,
    ) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            id: None,
            company_id,
            account_code: account_code.into(),
            account_name: account_name.into(),
            account_type,
            parent_account: None,
            status: STATUS_ACTIVE.to_string(),
            reference_code: Some(reference_code::generate_code("COA")),
            description: String::new(),
            attachments: Vec::new(),
            create_timestamp: now,
            modified_timestamp: now,
        }
    }

    pub fn normal_balance(&self) -> NormalBalance {
        self.account_type.normal_balance()
    }

    /// Whether the account can be posted to and shows up in pickers --
    /// the boolean view of `status` every filter/toggle in the app
    /// actually checks, so `status` never needs comparing against
    /// [`STATUS_ACTIVE`] by hand at each call site.
    pub fn is_active(&self) -> bool {
        self.status == STATUS_ACTIVE
    }

    pub fn set_active(&mut self, active: bool) {
        self.status = if active {
            STATUS_ACTIVE
        } else {
            STATUS_INACTIVE
        }
        .to_string();
    }
}

impl Validate for Account {
    fn validate(&self) -> AppResult<()> {
        if self.company_id <= 0 {
            return Err(AppError::Validation(
                "account must reference a valid company".into(),
            ));
        }
        require_non_empty("account code", &self.account_code)?;
        require_max_length("account code", &self.account_code, 20)?;
        require_non_empty("account name", &self.account_name)?;
        require_max_length("account name", &self.account_name, 200)?;
        require_max_length("account description", &self.description, 2000)?;
        if matches!(self.parent_account, Some(id) if id <= 0) {
            return Err(AppError::Validation(
                "account's parent_account, when set, must be a valid account".into(),
            ));
        }
        if self.parent_account == self.id && self.id.is_some() {
            return Err(AppError::Validation(
                "an account cannot be its own parent".into(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_balance_follows_account_type() {
        let cash = Account::new(1, "1000", "Cash", AccountType::Asset);
        assert_eq!(cash.normal_balance(), NormalBalance::Debit);

        let revenue = Account::new(1, "4000", "Sales Revenue", AccountType::Revenue);
        assert_eq!(revenue.normal_balance(), NormalBalance::Credit);
    }

    #[test]
    fn new_account_is_active_with_matching_create_and_modified_timestamps() {
        let account = Account::new(1, "1000", "Cash", AccountType::Asset);
        assert!(account.is_active());
        assert_eq!(account.create_timestamp, account.modified_timestamp);
    }

    #[test]
    fn set_active_toggles_the_status_string() {
        let mut account = Account::new(1, "1000", "Cash", AccountType::Asset);
        account.set_active(false);
        assert_eq!(account.status, STATUS_INACTIVE);
        assert!(!account.is_active());

        account.set_active(true);
        assert_eq!(account.status, STATUS_ACTIVE);
        assert!(account.is_active());
    }

    #[test]
    fn validate_rejects_blank_code_or_name() {
        assert!(Account::new(1, "", "Cash", AccountType::Asset)
            .validate()
            .is_err());
        assert!(Account::new(1, "1000", "", AccountType::Asset)
            .validate()
            .is_err());
    }

    #[test]
    fn validate_rejects_non_positive_company_id() {
        assert!(Account::new(0, "1000", "Cash", AccountType::Asset)
            .validate()
            .is_err());
    }

    #[test]
    fn validate_rejects_account_as_its_own_parent() {
        let mut account = Account::new(1, "1000", "Cash", AccountType::Asset);
        account.id = Some(5);
        account.parent_account = Some(5);
        assert!(account.validate().is_err());
    }

    #[test]
    fn validate_accepts_well_formed_account() {
        let account = Account::new(1, "1000", "Cash", AccountType::Asset);
        assert!(account.validate().is_ok());
    }
}
