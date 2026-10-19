//! A real-world bank account, linked to the GL asset account that
//! represents it in the chart of accounts.

use rust_decimal::Decimal;

use crate::backend::core::errors::{AppError, AppResult};
use crate::backend::core::validation::{require_max_length, require_non_empty, Validate};

/// Which side of the ledger increases the account's net balance.
///
/// Sign convention: a `Debit` account (an ordinary current/savings
/// account, an asset) nets `opening + debits - credits`; a `Credit`
/// account (an overdraft or credit line, a liability) nets
/// `opening + credits - debits` -- the same credit-minus-debit rule the
/// vendor (accounts payable) ledger uses. A positive net is always "in
/// the account's normal direction".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BankAccountType {
    Credit,
    #[default]
    Debit,
}

pub const BANK_ACCOUNT_TYPES: [BankAccountType; 2] =
    [BankAccountType::Credit, BankAccountType::Debit];

impl BankAccountType {
    pub fn as_str(self) -> &'static str {
        match self {
            BankAccountType::Credit => "Credit",
            BankAccountType::Debit => "Debit",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        BANK_ACCOUNT_TYPES
            .into_iter()
            .find(|kind| kind.as_str().eq_ignore_ascii_case(value.trim()))
    }

    /// How much one posting moves the net balance, per the convention above.
    pub fn net_movement(self, debit: Decimal, credit: Decimal) -> Decimal {
        match self {
            BankAccountType::Debit => debit - credit,
            BankAccountType::Credit => credit - debit,
        }
    }
}

impl std::fmt::Display for BankAccountType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BankAccount {
    pub id: Option<i64>,
    pub company_id: i64,
    /// The chart-of-accounts Asset account this bank account posts to.
    pub gl_account_id: i64,
    pub bank_name: String,
    pub account_number: String,
    pub currency_code: String,
    pub is_active: bool,
    pub parent_company: String,
    pub sub_company: String,
    pub account_type: BankAccountType,
    /// Balance carried in from before the first posted transaction, in
    /// the account's normal direction (see [`BankAccountType`]).
    pub opening_balance: Decimal,
}

impl BankAccount {
    pub fn new(
        company_id: i64,
        gl_account_id: i64,
        bank_name: impl Into<String>,
        account_number: impl Into<String>,
        currency_code: impl Into<String>,
    ) -> Self {
        Self {
            id: None,
            company_id,
            gl_account_id,
            bank_name: bank_name.into(),
            account_number: account_number.into(),
            currency_code: currency_code.into().to_ascii_uppercase(),
            is_active: true,
            parent_company: String::new(),
            sub_company: String::new(),
            account_type: BankAccountType::default(),
            opening_balance: Decimal::ZERO,
        }
    }

    /// The last four characters of the account number (all of it when
    /// shorter), for a masked display.
    pub fn account_number_tail(&self) -> String {
        let number = self.account_number.trim();
        let count = number.chars().count();
        number.chars().skip(count.saturating_sub(4)).collect()
    }

    /// The account number as compared for uniqueness: trimmed, ignoring
    /// case, spaces and dashes.
    pub fn normalized_account_number(&self) -> String {
        normalize_account_number(&self.account_number)
    }
}

pub fn normalize_account_number(value: &str) -> String {
    value
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .flat_map(char::to_uppercase)
        .collect()
}

impl Validate for BankAccount {
    fn validate(&self) -> AppResult<()> {
        if self.company_id <= 0 {
            return Err(AppError::Validation(
                "bank account must reference a valid company".into(),
            ));
        }
        if self.gl_account_id <= 0 {
            return Err(AppError::Validation(
                "bank account must reference a valid GL account".into(),
            ));
        }
        require_non_empty("bank name", &self.bank_name)?;
        require_max_length("bank name", &self.bank_name, 200)?;
        require_non_empty("account number", &self.account_number)?;
        require_max_length("account number", &self.account_number, 50)?;
        require_max_length("parent company", &self.parent_company, 200)?;
        require_max_length("sub company", &self.sub_company, 200)?;
        crate::backend::currency::currencies::validate_currency_code(&self.currency_code)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn validate_rejects_invalid_currency_code() {
        let account = BankAccount::new(1, 10, "First National", "123456789", "dollars");
        assert!(account.validate().is_err());
    }

    #[test]
    fn validate_accepts_a_well_formed_bank_account() {
        let account = BankAccount::new(1, 10, "First National", "123456789", "usd");
        assert!(account.validate().is_ok());
        assert_eq!(account.currency_code, "USD");
        assert_eq!(account.account_type, BankAccountType::Debit);
        assert_eq!(account.opening_balance, Decimal::ZERO);
    }

    #[test]
    fn account_type_round_trips_and_nets_in_its_normal_direction() {
        for kind in BANK_ACCOUNT_TYPES {
            assert_eq!(BankAccountType::parse(kind.as_str()), Some(kind));
        }
        assert_eq!(BankAccountType::parse("unknown"), None);
        assert_eq!(
            BankAccountType::Debit.net_movement(dec!(100), dec!(30)),
            dec!(70)
        );
        assert_eq!(
            BankAccountType::Credit.net_movement(dec!(100), dec!(30)),
            dec!(-70)
        );
    }

    #[test]
    fn the_tail_is_the_last_four_characters_and_numbers_normalize_for_uniqueness() {
        let account = BankAccount::new(1, 10, "First National", " PK36-HABB 0001 ", "PKR");
        assert_eq!(account.account_number_tail(), "0001");
        assert_eq!(account.normalized_account_number(), "PK36HABB0001");
        let short = BankAccount::new(1, 10, "First National", "12", "PKR");
        assert_eq!(short.account_number_tail(), "12");
    }
}
