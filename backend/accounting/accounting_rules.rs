//! Core double-entry accounting rules shared by the chart of accounts,
//! journal entries, and the general ledger: what an account type is, and
//! which side of a posting (debit or credit) increases its balance.
//! `chart_of_accounts.rs` classifies accounts with `AccountType`;
//! the transaction engine (Development Order step 6) uses
//! `AccountType::normal_balance` to turn postings into signed balance
//! movements.
//!
//! Also holds `AgingBucket`: the "how overdue is this" classification
//! shared by `accounts_receivable::aging` and `accounts_payable::aging`
//! (Development Order step 9) -- one rule, not duplicated between them.

use chrono::NaiveDate;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AgingBucket {
    /// Not yet due (or due today).
    Current,
    Days1To30,
    Days31To60,
    Days61To90,
    Over90,
}

/// Buckets a due date relative to `as_of` -- the same rule for a
/// receivable (owed to us) or a payable (owed by us).
pub fn aging_bucket_for(due_date: NaiveDate, as_of: NaiveDate) -> AgingBucket {
    let days_overdue = (as_of - due_date).num_days();
    if days_overdue <= 0 {
        AgingBucket::Current
    } else if days_overdue <= 30 {
        AgingBucket::Days1To30
    } else if days_overdue <= 60 {
        AgingBucket::Days31To60
    } else if days_overdue <= 90 {
        AgingBucket::Days61To90
    } else {
        AgingBucket::Over90
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AccountType {
    Asset,
    Liability,
    Equity,
    Revenue,
    Expense,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NormalBalance {
    Debit,
    Credit,
}

impl AccountType {
    /// The side (debit or credit) that *increases* this account type's
    /// balance. Assets and Expenses grow on the debit side; Liabilities,
    /// Equity, and Revenue grow on the credit side.
    pub fn normal_balance(self) -> NormalBalance {
        match self {
            AccountType::Asset | AccountType::Expense => NormalBalance::Debit,
            AccountType::Liability | AccountType::Equity | AccountType::Revenue => NormalBalance::Credit,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            AccountType::Asset => "asset",
            AccountType::Liability => "liability",
            AccountType::Equity => "equity",
            AccountType::Revenue => "revenue",
            AccountType::Expense => "expense",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "asset" => Some(AccountType::Asset),
            "liability" => Some(AccountType::Liability),
            "equity" => Some(AccountType::Equity),
            "revenue" => Some(AccountType::Revenue),
            "expense" => Some(AccountType::Expense),
            _ => None,
        }
    }
}

impl std::fmt::Display for AccountType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assets_and_expenses_have_a_normal_debit_balance() {
        assert_eq!(AccountType::Asset.normal_balance(), NormalBalance::Debit);
        assert_eq!(AccountType::Expense.normal_balance(), NormalBalance::Debit);
    }

    #[test]
    fn liabilities_equity_and_revenue_have_a_normal_credit_balance() {
        assert_eq!(AccountType::Liability.normal_balance(), NormalBalance::Credit);
        assert_eq!(AccountType::Equity.normal_balance(), NormalBalance::Credit);
        assert_eq!(AccountType::Revenue.normal_balance(), NormalBalance::Credit);
    }

    #[test]
    fn as_str_and_parse_round_trip_for_every_variant() {
        for account_type in [
            AccountType::Asset,
            AccountType::Liability,
            AccountType::Equity,
            AccountType::Revenue,
            AccountType::Expense,
        ] {
            assert_eq!(AccountType::parse(account_type.as_str()), Some(account_type));
        }
    }

    #[test]
    fn parse_rejects_unknown_values() {
        assert_eq!(AccountType::parse("not-a-type"), None);
    }

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn not_yet_due_or_due_today_is_current() {
        assert_eq!(aging_bucket_for(date(2026, 3, 1), date(2026, 2, 15)), AgingBucket::Current);
        assert_eq!(aging_bucket_for(date(2026, 3, 1), date(2026, 3, 1)), AgingBucket::Current);
    }

    #[test]
    fn aging_bucket_boundaries_are_inclusive_of_their_upper_edge() {
        assert_eq!(aging_bucket_for(date(2026, 1, 1), date(2026, 1, 31)), AgingBucket::Days1To30);
        assert_eq!(aging_bucket_for(date(2026, 1, 1), date(2026, 2, 1)), AgingBucket::Days31To60);
        assert_eq!(aging_bucket_for(date(2026, 1, 1), date(2026, 3, 3)), AgingBucket::Days61To90);
        assert_eq!(aging_bucket_for(date(2026, 1, 1), date(2026, 4, 2)), AgingBucket::Over90);
    }
}
