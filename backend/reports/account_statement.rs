//! A single customer's or vendor's statement: every charge and payment,
//! in date order, with a running balance -- the AR/AP equivalent of
//! `accounting::general_ledger`.

use chrono::NaiveDate;
use rust_decimal::Decimal;

#[derive(Debug, Clone, PartialEq)]
pub struct StatementLine {
    pub date: NaiveDate,
    pub reference: Option<String>,
    pub description: String,
    /// Positive for a charge (invoice/bill), negative for a payment.
    pub amount: Decimal,
    pub running_balance: Decimal,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AccountStatement {
    pub party_name: String,
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
    pub opening_balance: Decimal,
    pub lines: Vec<StatementLine>,
}

impl AccountStatement {
    pub fn closing_balance(&self) -> Decimal {
        self.lines.last().map(|l| l.running_balance).unwrap_or(self.opening_balance)
    }

    pub fn to_rows(&self) -> (Vec<&'static str>, Vec<Vec<String>>) {
        let mut rows = vec![vec!["".into(), "".into(), "Opening balance".into(), "".into(), self.opening_balance.to_string()]];
        for line in &self.lines {
            rows.push(vec![
                line.date.to_string(),
                line.reference.clone().unwrap_or_default(),
                line.description.clone(),
                line.amount.to_string(),
                line.running_balance.to_string(),
            ]);
        }
        (vec!["Date", "Reference", "Description", "Amount", "Balance"], rows)
    }
}

/// `events` must already be sorted chronologically -- (date, reference,
/// description, signed amount).
pub fn build(party_name: impl Into<String>, period_start: NaiveDate, period_end: NaiveDate, opening_balance: Decimal, events: &[(NaiveDate, Option<String>, String, Decimal)]) -> AccountStatement {
    let mut running_balance = opening_balance;
    let lines = events
        .iter()
        .map(|(date, reference, description, amount)| {
            running_balance += *amount;
            StatementLine {
                date: *date,
                reference: reference.clone(),
                description: description.clone(),
                amount: *amount,
                running_balance,
            }
        })
        .collect();

    AccountStatement {
        party_name: party_name.into(),
        period_start,
        period_end,
        opening_balance,
        lines,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn running_balance_accumulates_charges_and_payments() {
        let events = vec![
            (date(2026, 1, 5), Some("INV-1".to_string()), "Invoice".to_string(), dec!(1000)),
            (date(2026, 1, 20), None, "Payment received".to_string(), dec!(-400)),
        ];
        let statement = build("Acme Retail", date(2026, 1, 1), date(2026, 1, 31), Decimal::ZERO, &events);

        assert_eq!(statement.lines[0].running_balance, dec!(1000));
        assert_eq!(statement.lines[1].running_balance, dec!(600));
        assert_eq!(statement.closing_balance(), dec!(600));
    }

    #[test]
    fn closing_balance_with_no_activity_is_the_opening_balance() {
        let statement = build("Acme Retail", date(2026, 1, 1), date(2026, 1, 31), dec!(250), &[]);
        assert_eq!(statement.closing_balance(), dec!(250));
    }
}
