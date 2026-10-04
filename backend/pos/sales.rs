//! A point-of-sale sale: unlike a regular sales invoice (Development
//! Order step 10), it's paid in full at the point of sale -- no
//! receivable, just cash/card tender -- so its GL posting debits each
//! payment method's account directly instead of AR.

use chrono::NaiveDate;
use rust_decimal::Decimal;

use crate::backend::core::errors::{AppError, AppResult};
use crate::backend::core::validation::{require_non_empty, require_positive, Validate};
use crate::backend::pos::payments::{PaymentMethod, PosPayment};
use crate::backend::transactions::transaction::Transaction;
use crate::backend::transactions::transaction_lines::TransactionLine;
use crate::backend::transactions::transaction_types::TransactionType;

#[derive(Debug, Clone, PartialEq)]
pub struct PosSaleLine {
    pub product_id: i64,
    pub revenue_account_id: i64,
    pub description: String,
    pub quantity: Decimal,
    pub unit_price: Decimal,
}

impl PosSaleLine {
    pub fn new(
        product_id: i64,
        revenue_account_id: i64,
        description: impl Into<String>,
        quantity: Decimal,
        unit_price: Decimal,
    ) -> Self {
        Self {
            product_id,
            revenue_account_id,
            description: description.into(),
            quantity,
            unit_price,
        }
    }

    pub fn amount(&self) -> Decimal {
        self.quantity * self.unit_price
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PosSale {
    pub id: Option<i64>,
    pub company_id: i64,
    pub terminal_id: i64,
    pub session_id: i64,
    pub sale_date: NaiveDate,
    pub lines: Vec<PosSaleLine>,
    pub payments: Vec<PosPayment>,
    pub transaction_id: Option<i64>,
}

impl PosSale {
    pub fn new(company_id: i64, terminal_id: i64, session_id: i64, sale_date: NaiveDate) -> Self {
        Self {
            id: None,
            company_id,
            terminal_id,
            session_id,
            sale_date,
            lines: Vec::new(),
            payments: Vec::new(),
            transaction_id: None,
        }
    }

    pub fn total(&self) -> Decimal {
        self.lines.iter().map(PosSaleLine::amount).sum()
    }

    pub fn total_tendered(&self) -> Decimal {
        crate::backend::pos::payments::total_tendered(&self.payments)
    }

    /// Debits each payment's GL account for its tendered amount, credits
    /// each line's revenue account. `payment_accounts` maps a payment
    /// method to the GL account it clears into (a cash drawer account for
    /// `Cash`, a card clearing account for `Card`, ...).
    pub fn build_transaction(
        &self,
        payment_accounts: &[(PaymentMethod, i64)],
    ) -> AppResult<Transaction> {
        let mut transaction =
            Transaction::new(self.company_id, TransactionType::PosSale, self.sale_date);
        transaction.description = Some(format!("POS sale, terminal #{}", self.terminal_id));

        for payment in &self.payments {
            let account_id = payment_accounts
                .iter()
                .find(|(method, _)| *method == payment.method)
                .map(|(_, account_id)| *account_id)
                .ok_or_else(|| {
                    AppError::Validation(format!(
                        "no GL account configured for payment method {}",
                        payment.method.as_str()
                    ))
                })?;
            transaction
                .lines
                .push(TransactionLine::debit(account_id, payment.amount));
        }
        for line in &self.lines {
            transaction.lines.push(TransactionLine {
                description: Some(line.description.clone()),
                ..TransactionLine::credit(line.revenue_account_id, line.amount())
            });
        }

        Ok(transaction)
    }
}

impl Validate for PosSale {
    fn validate(&self) -> AppResult<()> {
        if self.company_id <= 0 {
            return Err(AppError::Validation(
                "POS sale must reference a valid company".into(),
            ));
        }
        if self.terminal_id <= 0 {
            return Err(AppError::Validation(
                "POS sale must reference a valid terminal".into(),
            ));
        }
        if self.session_id <= 0 {
            return Err(AppError::Validation(
                "POS sale must reference a valid session".into(),
            ));
        }
        if self.lines.is_empty() {
            return Err(AppError::Validation(
                "a POS sale must have at least one line".into(),
            ));
        }
        for line in &self.lines {
            require_non_empty("POS sale line description", &line.description)?;
            require_positive("POS sale line quantity", line.quantity)?;
            require_positive("POS sale line unit price", line.unit_price)?;
        }
        if self.payments.is_empty() {
            return Err(AppError::Validation(
                "a POS sale must have at least one payment".into(),
            ));
        }
        for payment in &self.payments {
            payment.validate()?;
        }
        if self.total_tendered() != self.total() {
            return Err(AppError::Validation(format!(
                "POS sale total {} does not match amount tendered {}",
                self.total(),
                self.total_tendered()
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn date() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 1, 15).unwrap()
    }

    fn sale_with_line_and_payment() -> PosSale {
        let mut sale = PosSale::new(1, 1, 1, date());
        sale.lines
            .push(PosSaleLine::new(10, 40, "Widget", dec!(2), dec!(50)));
        sale.payments
            .push(PosPayment::new(PaymentMethod::Cash, dec!(100)));
        sale
    }

    #[test]
    fn build_transaction_debits_payment_account_and_credits_revenue() {
        let sale = sale_with_line_and_payment();
        let transaction = sale
            .build_transaction(&[(PaymentMethod::Cash, 20)])
            .unwrap();
        assert!(transaction.is_balanced());
        assert_eq!(transaction.lines[0].account_id, 20);
        assert_eq!(transaction.lines[0].debit, dec!(100));
        assert_eq!(transaction.lines[1].account_id, 40);
        assert_eq!(transaction.lines[1].credit, dec!(100));
    }

    #[test]
    fn build_transaction_fails_without_a_configured_account_for_the_payment_method() {
        let sale = sale_with_line_and_payment();
        assert!(sale
            .build_transaction(&[(PaymentMethod::Card, 30)])
            .is_err());
    }

    #[test]
    fn validate_rejects_a_sale_where_tendered_does_not_match_total() {
        let mut sale = sale_with_line_and_payment();
        sale.payments[0].amount = dec!(50);
        assert!(sale.validate().is_err());
    }

    #[test]
    fn validate_accepts_a_well_formed_split_payment_sale() {
        let mut sale = sale_with_line_and_payment();
        sale.payments[0].amount = dec!(60);
        sale.payments
            .push(PosPayment::new(PaymentMethod::Card, dec!(40)));
        assert!(sale.validate().is_ok());
    }
}
