//! The balance sheet: assets, liabilities, and equity as of a date, with
//! the current period's net income folded into equity (so the sheet
//! balances even before a formal period-closing entry has run).

use chrono::NaiveDate;
use rust_decimal::Decimal;

use crate::backend::accounting::accounting_rules::{AccountType, NormalBalance};

#[derive(Debug, Clone, PartialEq)]
pub struct BalanceSheetLine {
    pub account_id: i64,
    pub account_code: String,
    pub account_name: String,
    /// Always shown positive-on-its-normal-side: an asset's debit
    /// balance, a liability/equity account's credit balance.
    pub balance: Decimal,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BalanceSheet {
    pub as_of: NaiveDate,
    pub assets: Vec<BalanceSheetLine>,
    pub liabilities: Vec<BalanceSheetLine>,
    pub equity: Vec<BalanceSheetLine>,
    /// Net income for the current, not-yet-closed period.
    pub current_year_earnings: Decimal,
}

impl BalanceSheet {
    pub fn total_assets(&self) -> Decimal {
        self.assets.iter().map(|l| l.balance).sum()
    }

    pub fn total_liabilities(&self) -> Decimal {
        self.liabilities.iter().map(|l| l.balance).sum()
    }

    pub fn total_equity(&self) -> Decimal {
        self.equity.iter().map(|l| l.balance).sum::<Decimal>() + self.current_year_earnings
    }

    pub fn is_balanced(&self) -> bool {
        self.total_assets() == self.total_liabilities() + self.total_equity()
    }

    pub fn to_rows(&self) -> (Vec<&'static str>, Vec<Vec<String>>) {
        let mut rows = Vec::new();
        for (heading, lines) in [
            ("Assets", &self.assets),
            ("Liabilities", &self.liabilities),
            ("Equity", &self.equity),
        ] {
            rows.push(vec![heading.to_string(), String::new(), String::new()]);
            for line in lines {
                rows.push(vec![
                    line.account_code.clone(),
                    line.account_name.clone(),
                    line.balance.to_string(),
                ]);
            }
        }
        rows.push(vec![
            "Equity".into(),
            "Current Year Earnings".into(),
            self.current_year_earnings.to_string(),
        ]);
        (vec!["Code", "Account", "Balance"], rows)
    }
}

/// `balances` is (account_id, code, name, account_type, trial-balance-style
/// net movement: positive = debit, negative = credit) for every Asset,
/// Liability, and Equity account as of `as_of`.
pub fn build(
    as_of: NaiveDate,
    balances: &[(i64, String, String, AccountType, Decimal)],
    current_year_earnings: Decimal,
) -> BalanceSheet {
    let mut sheet = BalanceSheet {
        as_of,
        assets: Vec::new(),
        liabilities: Vec::new(),
        equity: Vec::new(),
        current_year_earnings,
    };

    for (account_id, code, name, account_type, net) in balances {
        let balance = match account_type.normal_balance() {
            NormalBalance::Debit => *net,
            NormalBalance::Credit => -*net,
        };
        let line = BalanceSheetLine {
            account_id: *account_id,
            account_code: code.clone(),
            account_name: name.clone(),
            balance,
        };
        match account_type {
            AccountType::Asset => sheet.assets.push(line),
            AccountType::Liability => sheet.liabilities.push(line),
            AccountType::Equity => sheet.equity.push(line),
            AccountType::Revenue | AccountType::Expense => {}
        }
    }

    sheet
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn date() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 12, 31).unwrap()
    }

    #[test]
    fn balances_when_assets_equal_liabilities_plus_equity_plus_earnings() {
        let balances = vec![
            (
                1,
                "1000".to_string(),
                "Cash".to_string(),
                AccountType::Asset,
                dec!(15000),
            ),
            (
                2,
                "2000".to_string(),
                "Accounts Payable".to_string(),
                AccountType::Liability,
                dec!(-3000),
            ),
            (
                3,
                "3000".to_string(),
                "Owner's Capital".to_string(),
                AccountType::Equity,
                dec!(-10000),
            ),
        ];
        let sheet = build(date(), &balances, dec!(2000));

        assert_eq!(sheet.total_assets(), dec!(15000));
        assert_eq!(sheet.total_liabilities(), dec!(3000));
        assert_eq!(sheet.total_equity(), dec!(12000)); // 10000 capital + 2000 earnings
        assert!(sheet.is_balanced());
    }

    #[test]
    fn revenue_and_expense_accounts_are_excluded() {
        let balances = vec![
            (
                1,
                "1000".to_string(),
                "Cash".to_string(),
                AccountType::Asset,
                dec!(1000),
            ),
            (
                4,
                "4000".to_string(),
                "Sales Revenue".to_string(),
                AccountType::Revenue,
                dec!(-5000),
            ),
        ];
        let sheet = build(date(), &balances, Decimal::ZERO);
        assert!(sheet.assets.iter().all(|l| l.account_id != 4));
        assert_eq!(sheet.assets.len(), 1);
    }
}
