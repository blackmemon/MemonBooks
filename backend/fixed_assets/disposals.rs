//! Disposing of a fixed asset: clear its cost and accumulated
//! depreciation off the books, record any proceeds, and recognize the
//! gain or loss (proceeds vs. remaining book value) on disposal.

use chrono::NaiveDate;
use rust_decimal::Decimal;

use crate::backend::transactions::transaction::Transaction;
use crate::backend::transactions::transaction_lines::TransactionLine;
use crate::backend::transactions::transaction_types::TransactionType;

pub struct AssetDisposal {
    pub company_id: i64,
    pub asset_name: String,
    pub disposal_date: NaiveDate,
    pub acquisition_cost: Decimal,
    pub accumulated_depreciation: Decimal,
    /// What was received for the asset, if anything (0 for a scrapped asset).
    pub proceeds: Decimal,
}

impl AssetDisposal {
    pub fn book_value(&self) -> Decimal {
        self.acquisition_cost - self.accumulated_depreciation
    }

    /// Positive is a gain, negative is a loss.
    pub fn gain_or_loss(&self) -> Decimal {
        self.proceeds - self.book_value()
    }

    /// Clears the asset's cost and accumulated depreciation, records
    /// proceeds against `proceeds_account_id` (typically cash), and
    /// posts the gain/loss to `gain_loss_account_id`. Always balances --
    /// see the module-level reasoning in the repository that calls this.
    pub fn build_transaction(
        &self,
        asset_gl_account_id: i64,
        accumulated_depreciation_gl_account_id: i64,
        proceeds_account_id: i64,
        gain_loss_account_id: i64,
    ) -> Transaction {
        let mut transaction = Transaction::new(
            self.company_id,
            TransactionType::ManualJournal,
            self.disposal_date,
        );
        transaction.description = Some(format!("Disposal: {}", self.asset_name));

        if self.accumulated_depreciation > Decimal::ZERO {
            transaction.lines.push(TransactionLine::debit(
                accumulated_depreciation_gl_account_id,
                self.accumulated_depreciation,
            ));
        }
        if self.proceeds > Decimal::ZERO {
            transaction
                .lines
                .push(TransactionLine::debit(proceeds_account_id, self.proceeds));
        }
        transaction.lines.push(TransactionLine::credit(
            asset_gl_account_id,
            self.acquisition_cost,
        ));

        let gain_or_loss = self.gain_or_loss();
        if gain_or_loss > Decimal::ZERO {
            transaction
                .lines
                .push(TransactionLine::credit(gain_loss_account_id, gain_or_loss));
        } else if gain_or_loss < Decimal::ZERO {
            transaction
                .lines
                .push(TransactionLine::debit(gain_loss_account_id, -gain_or_loss));
        }

        transaction
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn date() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 6, 30).unwrap()
    }

    #[test]
    fn selling_above_book_value_is_a_gain_and_stays_balanced() {
        let disposal = AssetDisposal {
            company_id: 1,
            asset_name: "Delivery Van".into(),
            disposal_date: date(),
            acquisition_cost: dec!(20000),
            accumulated_depreciation: dec!(15000),
            proceeds: dec!(6000),
        };
        // book value = 5000, proceeds = 6000 -> gain of 1000.
        assert_eq!(disposal.gain_or_loss(), dec!(1000));

        let transaction = disposal.build_transaction(10, 11, 12, 13);
        assert!(transaction.is_balanced());
        assert_eq!(transaction.total_debits(), dec!(21000)); // 15000 + 6000
        assert_eq!(transaction.total_credits(), dec!(21000)); // 20000 + 1000
    }

    #[test]
    fn selling_below_book_value_is_a_loss_and_stays_balanced() {
        let disposal = AssetDisposal {
            company_id: 1,
            asset_name: "Delivery Van".into(),
            disposal_date: date(),
            acquisition_cost: dec!(20000),
            accumulated_depreciation: dec!(15000),
            proceeds: dec!(2000),
        };
        // book value = 5000, proceeds = 2000 -> loss of 3000.
        assert_eq!(disposal.gain_or_loss(), dec!(-3000));

        let transaction = disposal.build_transaction(10, 11, 12, 13);
        assert!(transaction.is_balanced());
    }

    #[test]
    fn scrapping_a_fully_depreciated_asset_with_no_proceeds_is_a_total_write_off() {
        let disposal = AssetDisposal {
            company_id: 1,
            asset_name: "Old Printer".into(),
            disposal_date: date(),
            acquisition_cost: dec!(500),
            accumulated_depreciation: dec!(500),
            proceeds: Decimal::ZERO,
        };
        assert_eq!(disposal.gain_or_loss(), Decimal::ZERO);

        let transaction = disposal.build_transaction(10, 11, 12, 13);
        assert!(transaction.is_balanced());
        // no proceeds and no gain/loss line -- just clearing depreciation against cost.
        assert_eq!(transaction.lines.len(), 2);
    }
}
