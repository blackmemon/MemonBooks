//! A company's chart of accounts: the set of ledger accounts everything
//! else (journal entries, the general ledger, financial reports) posts
//! against or derives from.

use crate::backend::accounting::accounting_rules::{AccountType, NormalBalance};
use crate::backend::core::errors::{AppError, AppResult};
use crate::backend::core::reference_code;
use crate::backend::core::validation::{require_max_length, require_non_empty, Validate};

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
    pub code: String,
    pub name: String,
    pub account_type: AccountType,
    /// Parent account, for a hierarchical chart of accounts (e.g. "1000
    /// Cash" rolling up into a "1000-1999 Current Assets" heading).
    pub parent_id: Option<i64>,
    pub is_active: bool,
    /// Auto-generated, collision-safe internal reference id (`COA-
    /// YYYYMMDDHHMMSSmmm`) -- see `core::reference_code`. `None` only
    /// for accounts created before this field existed; every account
    /// created through `Account::new` gets one automatically.
    pub reference_code: Option<String>,
}

impl Account {
    pub fn new(company_id: i64, code: impl Into<String>, name: impl Into<String>, account_type: AccountType) -> Self {
        Self {
            id: None,
            company_id,
            code: code.into(),
            name: name.into(),
            account_type,
            parent_id: None,
            is_active: true,
            reference_code: Some(reference_code::generate_code("COA")),
        }
    }

    pub fn normal_balance(&self) -> NormalBalance {
        self.account_type.normal_balance()
    }
}

impl Validate for Account {
    fn validate(&self) -> AppResult<()> {
        if self.company_id <= 0 {
            return Err(AppError::Validation("account must reference a valid company".into()));
        }
        require_non_empty("account code", &self.code)?;
        require_max_length("account code", &self.code, 20)?;
        require_non_empty("account name", &self.name)?;
        require_max_length("account name", &self.name, 200)?;
        if matches!(self.parent_id, Some(id) if id <= 0) {
            return Err(AppError::Validation(
                "account's parent_id, when set, must be a valid account".into(),
            ));
        }
        if self.parent_id == self.id && self.id.is_some() {
            return Err(AppError::Validation("an account cannot be its own parent".into()));
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
    fn validate_rejects_blank_code_or_name() {
        assert!(Account::new(1, "", "Cash", AccountType::Asset).validate().is_err());
        assert!(Account::new(1, "1000", "", AccountType::Asset).validate().is_err());
    }

    #[test]
    fn validate_rejects_non_positive_company_id() {
        assert!(Account::new(0, "1000", "Cash", AccountType::Asset).validate().is_err());
    }

    #[test]
    fn validate_rejects_account_as_its_own_parent() {
        let mut account = Account::new(1, "1000", "Cash", AccountType::Asset);
        account.id = Some(5);
        account.parent_id = Some(5);
        assert!(account.validate().is_err());
    }

    #[test]
    fn validate_accepts_well_formed_account() {
        let account = Account::new(1, "1000", "Cash", AccountType::Asset);
        assert!(account.validate().is_ok());
    }
}
