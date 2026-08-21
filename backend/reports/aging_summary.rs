//! Combined AR + AP aging: thin composition of
//! `accounts_receivable::aging` and `accounts_payable::aging`
//! (Development Order step 9) into one company-wide view.

use chrono::NaiveDate;

use crate::backend::accounting::accounts_payable::aging::PayablesAgingSummary;
use crate::backend::accounting::accounts_receivable::aging::ReceivablesAgingSummary;

#[derive(Debug, Clone, PartialEq)]
pub struct AgingSummaryReport {
    pub as_of: NaiveDate,
    pub receivables: ReceivablesAgingSummary,
    pub payables: PayablesAgingSummary,
}

impl AgingSummaryReport {
    pub fn to_rows(&self) -> (Vec<&'static str>, Vec<Vec<String>>) {
        let headers = vec!["Bucket", "Receivables", "Payables"];
        let rows = vec![
            vec!["Current".into(), self.receivables.current.to_string(), self.payables.current.to_string()],
            vec!["1-30 days".into(), self.receivables.days_1_to_30.to_string(), self.payables.days_1_to_30.to_string()],
            vec!["31-60 days".into(), self.receivables.days_31_to_60.to_string(), self.payables.days_31_to_60.to_string()],
            vec!["61-90 days".into(), self.receivables.days_61_to_90.to_string(), self.payables.days_61_to_90.to_string()],
            vec!["Over 90 days".into(), self.receivables.over_90.to_string(), self.payables.over_90.to_string()],
            vec!["Total".into(), self.receivables.total().to_string(), self.payables.total().to_string()],
        ];
        (headers, rows)
    }
}

pub fn build(as_of: NaiveDate, receivables: ReceivablesAgingSummary, payables: PayablesAgingSummary) -> AgingSummaryReport {
    AgingSummaryReport { as_of, receivables, payables }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_rows_includes_a_total_row() {
        let report = build(NaiveDate::from_ymd_opt(2026, 1, 31).unwrap(), ReceivablesAgingSummary::default(), PayablesAgingSummary::default());
        let (headers, rows) = report.to_rows();
        assert_eq!(headers.len(), 3);
        assert_eq!(rows.last().unwrap()[0], "Total");
    }
}
