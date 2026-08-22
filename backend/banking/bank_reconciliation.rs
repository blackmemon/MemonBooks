//! The reconciliation math: does the bank statement, adjusted for
//! deposits/withdrawals the bank hasn't cleared yet, agree with the
//! book (GL) balance?

use rust_decimal::Decimal;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReconciliationSummary {
    /// The GL balance for this bank account as of the reconciliation date.
    pub book_balance: Decimal,
    /// The balance per the bank statement.
    pub statement_balance: Decimal,
    /// Posted deposits not yet cleared on the statement.
    pub deposits_in_transit: Decimal,
    /// Posted withdrawals not yet cleared on the statement.
    pub outstanding_withdrawals: Decimal,
}

impl ReconciliationSummary {
    /// Standard formula: statement balance + deposits in transit -
    /// outstanding withdrawals = what the statement balance *should*
    /// become once everything clears.
    pub fn adjusted_statement_balance(&self) -> Decimal {
        self.statement_balance + self.deposits_in_transit - self.outstanding_withdrawals
    }

    /// True once the adjusted statement balance agrees with the books --
    /// meaning every difference between them is just timing (items not
    /// yet cleared), not an actual discrepancy.
    pub fn is_reconciled(&self) -> bool {
        self.adjusted_statement_balance() == self.book_balance
    }

    pub fn discrepancy(&self) -> Decimal {
        self.book_balance - self.adjusted_statement_balance()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn reconciled_when_adjusted_statement_matches_book_balance() {
        let summary = ReconciliationSummary {
            book_balance: dec!(1200),
            statement_balance: dec!(1000),
            deposits_in_transit: dec!(300),
            outstanding_withdrawals: dec!(100),
        };
        assert_eq!(summary.adjusted_statement_balance(), dec!(1200));
        assert!(summary.is_reconciled());
        assert_eq!(summary.discrepancy(), Decimal::ZERO);
    }

    #[test]
    fn not_reconciled_when_balances_disagree() {
        let summary = ReconciliationSummary {
            book_balance: dec!(1200),
            statement_balance: dec!(1000),
            deposits_in_transit: dec!(300),
            outstanding_withdrawals: dec!(50), // should have been 100
        };
        assert!(!summary.is_reconciled());
        assert_eq!(summary.discrepancy(), dec!(-50));
    }

    #[test]
    fn reconciled_with_no_items_in_transit_when_statement_already_matches_book() {
        let summary = ReconciliationSummary {
            book_balance: dec!(500),
            statement_balance: dec!(500),
            deposits_in_transit: Decimal::ZERO,
            outstanding_withdrawals: Decimal::ZERO,
        };
        assert!(summary.is_reconciled());
        assert_eq!(summary.discrepancy(), Decimal::ZERO);
    }

    #[test]
    fn positive_discrepancy_means_the_books_show_more_than_the_adjusted_statement() {
        // Book balance overstates what the bank will actually show once
        // outstanding items clear -- e.g. a posted deposit that was
        // never actually deposited.
        let summary = ReconciliationSummary {
            book_balance: dec!(1000),
            statement_balance: dec!(700),
            deposits_in_transit: Decimal::ZERO,
            outstanding_withdrawals: Decimal::ZERO,
        };
        assert!(!summary.is_reconciled());
        assert_eq!(summary.discrepancy(), dec!(300));
    }

    #[test]
    fn reconciles_correctly_with_a_negative_book_balance() {
        // An overdrawn account: outstanding withdrawals bring the
        // statement down to match a negative book balance.
        let summary = ReconciliationSummary {
            book_balance: dec!(-200),
            statement_balance: dec!(300),
            deposits_in_transit: Decimal::ZERO,
            outstanding_withdrawals: dec!(500),
        };
        assert!(summary.is_reconciled());
    }
}
