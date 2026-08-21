//! The income statement (profit & loss): revenue and expense *movement
//! within a period*, unlike the trial balance's cumulative-to-date view.
//! Reuses `period_closing::ClosingBalance`'s shape (account_id,
//! account_type, net_balance) since it's exactly the same data a
//! period-end closing entry consumes.

use chrono::NaiveDate;
use rust_decimal::Decimal;

use crate::backend::accounting::accounting_rules::AccountType;
use crate::backend::accounting::period_closing::ClosingBalance;

#[derive(Debug, Clone, PartialEq)]
pub struct ProfitAndLossLine {
    pub account_id: i64,
    pub account_code: String,
    pub account_name: String,
    pub amount: Decimal,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProfitAndLoss {
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
    pub revenue: Vec<ProfitAndLossLine>,
    pub expenses: Vec<ProfitAndLossLine>,
}

impl ProfitAndLoss {
    pub fn total_revenue(&self) -> Decimal {
        self.revenue.iter().map(|l| l.amount).sum()
    }

    pub fn total_expenses(&self) -> Decimal {
        self.expenses.iter().map(|l| l.amount).sum()
    }

    pub fn net_income(&self) -> Decimal {
        self.total_revenue() - self.total_expenses()
    }

    pub fn to_rows(&self) -> (Vec<&'static str>, Vec<Vec<String>>) {
        let mut rows = Vec::new();
        rows.push(vec!["Revenue".into(), String::new(), String::new()]);
        for line in &self.revenue {
            rows.push(vec![line.account_code.clone(), line.account_name.clone(), line.amount.to_string()]);
        }
        rows.push(vec!["Expenses".into(), String::new(), String::new()]);
        for line in &self.expenses {
            rows.push(vec![line.account_code.clone(), line.account_name.clone(), line.amount.to_string()]);
        }
        rows.push(vec!["Net Income".into(), String::new(), self.net_income().to_string()]);
        (vec!["Code", "Account", "Amount"], rows)
    }
}

/// `balances` is each Revenue/Expense account's net movement for the
/// period (trial-balance convention: positive = debit, negative =
/// credit -- so a healthy revenue account arrives negative here).
pub fn build(period_start: NaiveDate, period_end: NaiveDate, accounts: &[(String, String)], balances: &[ClosingBalance]) -> ProfitAndLoss {
    let mut report = ProfitAndLoss {
        period_start,
        period_end,
        revenue: Vec::new(),
        expenses: Vec::new(),
    };

    for (index, balance) in balances.iter().enumerate() {
        let (code, name) = accounts.get(index).cloned().unwrap_or_default();
        // Revenue's normal balance is credit, so its net movement is
        // usually negative here; flip it so the report shows a positive
        // revenue figure. Expense's normal debit balance is already
        // positive as-is.
        let amount = match balance.account_type {
            AccountType::Revenue => -balance.net_balance,
            AccountType::Expense => balance.net_balance,
            _ => continue,
        };
        let line = ProfitAndLossLine {
            account_id: balance.account_id,
            account_code: code,
            account_name: name,
            amount,
        };
        match balance.account_type {
            AccountType::Revenue => report.revenue.push(line),
            AccountType::Expense => report.expenses.push(line),
            _ => {}
        }
    }

    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn net_income_is_revenue_minus_expenses() {
        let balances = vec![
            ClosingBalance { account_id: 1, account_type: AccountType::Revenue, net_balance: dec!(-5000) },
            ClosingBalance { account_id: 2, account_type: AccountType::Expense, net_balance: dec!(3000) },
        ];
        let accounts = vec![("4000".to_string(), "Sales Revenue".to_string()), ("6000".to_string(), "Rent Expense".to_string())];
        let report = build(date(2026, 1, 1), date(2026, 1, 31), &accounts, &balances);

        assert_eq!(report.total_revenue(), dec!(5000));
        assert_eq!(report.total_expenses(), dec!(3000));
        assert_eq!(report.net_income(), dec!(2000));
    }
}
