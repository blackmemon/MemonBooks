//! The trial balance: every account's net balance as of a date, shown on
//! whichever side (debit or credit) it actually falls on -- not
//! necessarily the account's "normal" side, since a contra account (or
//! simply an overdrawn one) can carry an abnormal balance. Total debits
//! must equal total credits; because every posting that reached the
//! ledger already passed the transaction engine's balance check
//! (Section 4's hard invariant), this always holds and doubles as a
//! sanity check on the ledger itself.

use rust_decimal::Decimal;

#[derive(Debug, Clone, PartialEq)]
pub struct TrialBalanceLine {
    pub account_id: i64,
    pub account_code: String,
    pub account_name: String,
    pub debit: Decimal,
    pub credit: Decimal,
}

impl TrialBalanceLine {
    /// Builds a line from an account's net movement (total debits minus
    /// total credits): a non-negative net is a debit balance, negative is
    /// a credit balance.
    pub fn from_net_movement(account_id: i64, account_code: impl Into<String>, account_name: impl Into<String>, net: Decimal) -> Self {
        let (debit, credit) = if net >= Decimal::ZERO {
            (net, Decimal::ZERO)
        } else {
            (Decimal::ZERO, -net)
        };
        Self {
            account_id,
            account_code: account_code.into(),
            account_name: account_name.into(),
            debit,
            credit,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TrialBalance {
    pub as_of: chrono::NaiveDate,
    pub lines: Vec<TrialBalanceLine>,
}

impl TrialBalance {
    pub fn total_debits(&self) -> Decimal {
        self.lines.iter().map(|line| line.debit).sum()
    }

    pub fn total_credits(&self) -> Decimal {
        self.lines.iter().map(|line| line.credit).sum()
    }

    pub fn is_balanced(&self) -> bool {
        self.total_debits() == self.total_credits()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn non_negative_net_is_a_debit_balance() {
        let line = TrialBalanceLine::from_net_movement(1, "1000", "Cash", dec!(150));
        assert_eq!(line.debit, dec!(150));
        assert_eq!(line.credit, dec!(0));
    }

    #[test]
    fn negative_net_is_a_credit_balance() {
        let line = TrialBalanceLine::from_net_movement(1, "2000", "Accounts Payable", dec!(-75));
        assert_eq!(line.debit, dec!(0));
        assert_eq!(line.credit, dec!(75));
    }

    #[test]
    fn zero_net_is_a_zero_debit_balance() {
        let line = TrialBalanceLine::from_net_movement(1, "1000", "Cash", dec!(0));
        assert_eq!(line.debit, dec!(0));
        assert_eq!(line.credit, dec!(0));
    }

    #[test]
    fn is_balanced_reflects_totals() {
        let trial_balance = TrialBalance {
            as_of: chrono::NaiveDate::from_ymd_opt(2026, 1, 31).unwrap(),
            lines: vec![
                TrialBalanceLine::from_net_movement(1, "1000", "Cash", dec!(100)),
                TrialBalanceLine::from_net_movement(2, "3000", "Owner's Capital", dec!(-100)),
            ],
        };
        assert!(trial_balance.is_balanced());
    }

    #[test]
    fn is_balanced_false_when_totals_differ() {
        let trial_balance = TrialBalance {
            as_of: chrono::NaiveDate::from_ymd_opt(2026, 1, 31).unwrap(),
            lines: vec![
                TrialBalanceLine::from_net_movement(1, "1000", "Cash", dec!(100)),
                TrialBalanceLine::from_net_movement(2, "3000", "Owner's Capital", dec!(-90)),
            ],
        };
        assert!(!trial_balance.is_balanced());
    }
}
