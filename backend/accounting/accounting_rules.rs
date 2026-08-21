//! Core double-entry accounting rules shared by the chart of accounts,
//! journal entries, and the general ledger: what an account type is, and
//! which side of a posting (debit or credit) increases its balance.
//! `chart_of_accounts.rs` classifies accounts with `AccountType`;
//! the transaction engine (Development Order step 6) uses
//! `AccountType::normal_balance` to turn postings into signed balance
//! movements.

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
}
