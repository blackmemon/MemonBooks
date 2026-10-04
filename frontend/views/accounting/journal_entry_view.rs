//! Create and post a simple two-line manual journal entry (one debit
//! account, one credit account, one amount). Multi-line entries are
//! fully supported by the domain layer (`accounting::journal_entries`)
//! even though this first-pass screen only exposes the common
//! single-debit/single-credit case.

use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;

use iced::widget::{column, container, pick_list, scrollable, text, text_input};
use iced::{Element, Length};

use crate::backend::accounting::chart_of_accounts::Account;
use crate::backend::accounting::journal_entries::JournalEntry;
use crate::backend::services::accounting_service::AccountingService;
use crate::frontend::components::common::page_header;
use crate::frontend::styles::colors::Theme;
use crate::frontend::styles::dropdown;
use crate::frontend::styles::inputs;
use crate::frontend::styles::spacing;
use crate::frontend::views::Ctx;

#[derive(Debug, Clone, PartialEq)]
pub struct AccountOption {
    id: i64,
    label: String,
}

impl std::fmt::Display for AccountOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.label)
    }
}

#[derive(Debug, Clone, Default)]
pub struct State {
    accounts: Vec<AccountOption>,
    debit_account: Option<AccountOption>,
    credit_account: Option<AccountOption>,
    amount_input: String,
    memo_input: String,
    message: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    DebitAccountSelected(AccountOption),
    CreditAccountSelected(AccountOption),
    AmountChanged(String),
    MemoChanged(String),
    Post,
}

fn to_option(account: &Account) -> AccountOption {
    AccountOption {
        id: account.id.unwrap_or_default(),
        label: format!("{} - {}", account.account_code, account.account_name),
    }
}

pub fn load(ctx: &Ctx) -> State {
    let service = AccountingService::new(ctx.db.clone());
    match service.list_accounts(ctx.company_id) {
        Ok(accounts) => State {
            accounts: accounts.iter().map(to_option).collect(),
            ..Default::default()
        },
        Err(err) => State {
            error: Some(err.to_string()),
            ..Default::default()
        },
    }
}

pub fn update(state: &mut State, message: Message, ctx: &Ctx) {
    match message {
        Message::DebitAccountSelected(account) => state.debit_account = Some(account),
        Message::CreditAccountSelected(account) => state.credit_account = Some(account),
        Message::AmountChanged(value) => state.amount_input = value,
        Message::MemoChanged(value) => state.memo_input = value,
        Message::Post => {
            state.message = None;
            state.error = None;

            let (Some(debit), Some(credit)) = (&state.debit_account, &state.credit_account) else {
                state.error = Some("choose both a debit and a credit account".into());
                return;
            };
            let amount: Decimal = match state.amount_input.parse() {
                Ok(amount) => amount,
                Err(_) => {
                    state.error = Some("amount is not a valid number".into());
                    return;
                }
            };

            let today: NaiveDate = Utc::now().date_naive();
            let entry = JournalEntry::new(ctx.company_id, today)
                .with_memo(state.memo_input.clone())
                .debit(debit.id, amount)
                .credit(credit.id, amount)
                .into_transaction();

            let service = AccountingService::new(ctx.db.clone());
            match service.post_transaction(&entry) {
                Ok(posted) => {
                    state.message = Some(format!(
                        "Posted transaction #{}.",
                        posted.id.unwrap_or_default()
                    ));
                    state.amount_input.clear();
                    state.memo_input.clear();
                }
                Err(err) => state.error = Some(err.to_string()),
            }
        }
    }
}

pub fn view(state: &State, theme: Theme) -> Element<'_, Message> {
    let header = page_header::view(
        theme,
        "New Journal Entry",
        "Record a single manual debit and credit.",
        "Post entry",
        Message::Post,
    );

    let form = column![
        text("Debit account"),
        pick_list(
            state.accounts.as_slice(),
            state.debit_account.clone(),
            Message::DebitAccountSelected
        )
        .style(dropdown::style(theme))
        .placeholder("Select account"),
        text("Credit account"),
        pick_list(
            state.accounts.as_slice(),
            state.credit_account.clone(),
            Message::CreditAccountSelected
        )
        .style(dropdown::style(theme))
        .placeholder("Select account"),
        text("Amount"),
        text_input("0.00", &state.amount_input)
            .style(inputs::text(theme))
            .on_input(Message::AmountChanged)
            .padding(spacing::SM),
        text("Memo"),
        text_input("Description", &state.memo_input)
            .style(inputs::text(theme))
            .on_input(Message::MemoChanged)
            .padding(spacing::SM),
    ]
    .spacing(spacing::SPACE_10);

    let mut content = column![header, form]
        .spacing(spacing::SPACE_20)
        .padding(spacing::LG)
        .width(Length::Fill);

    if let Some(message) = &state.message {
        content = content.push(text(message.clone()).color(theme.positive_front()));
    }
    if let Some(error) = &state.error {
        content = content.push(text(error.clone()).color(theme.negative_front()));
    }

    container(scrollable(content).height(Length::Fill))
        .width(Length::Fill)
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::accounting::accounting_rules::AccountType;
    use crate::backend::accounting::fiscal_periods::FiscalPeriod;
    use crate::backend::organization::companies::Company;
    use crate::backend::repositories::accounting_repository::{self, AccountingRepository};
    use crate::backend::repositories::organization_repository::{self, OrganizationRepository};
    use crate::database::database::Database;
    use chrono::NaiveDate;
    use std::path::PathBuf;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn ctx_with_two_accounts() -> (Ctx, AccountOption, AccountOption) {
        let db = Database::open_in_memory().unwrap();
        db.run_migrations(organization_repository::MIGRATIONS)
            .unwrap();
        db.run_migrations(accounting_repository::MIGRATIONS)
            .unwrap();
        db.run_migrations(crate::backend::audit::audit_logs::MIGRATIONS)
            .unwrap();

        let org_repo = OrganizationRepository::new(db.clone());
        let company_id = org_repo
            .create_company(&Company::new("Acme Trading Co", "USD"))
            .unwrap()
            .id
            .unwrap();

        let accounting_repo = AccountingRepository::new(db.clone());
        accounting_repo
            .create_fiscal_period(&FiscalPeriod::new(
                company_id,
                "Open Period",
                date(2026, 1, 1),
                date(2030, 12, 31),
            ))
            .unwrap();
        let cash = accounting_repo
            .create_account(&Account::new(
                company_id,
                "1000",
                "Cash",
                AccountType::Asset,
            ))
            .unwrap();
        let capital = accounting_repo
            .create_account(&Account::new(
                company_id,
                "3000",
                "Owner's Capital",
                AccountType::Equity,
            ))
            .unwrap();

        let ctx = Ctx {
            db,
            company_id,
            current_user_id: None,
            export_dir: PathBuf::from("."),
            uploads_dir: PathBuf::from("."),
        };
        (ctx, to_option(&cash), to_option(&capital))
    }

    #[test]
    fn field_changed_messages_update_the_corresponding_field() {
        let (ctx, cash, capital) = ctx_with_two_accounts();
        let mut state = State::default();

        update(
            &mut state,
            Message::DebitAccountSelected(cash.clone()),
            &ctx,
        );
        update(
            &mut state,
            Message::CreditAccountSelected(capital.clone()),
            &ctx,
        );
        update(&mut state, Message::AmountChanged("500".into()), &ctx);
        update(
            &mut state,
            Message::MemoChanged("Owner contribution".into()),
            &ctx,
        );

        assert_eq!(state.debit_account, Some(cash));
        assert_eq!(state.credit_account, Some(capital));
        assert_eq!(state.amount_input, "500");
        assert_eq!(state.memo_input, "Owner contribution");
    }

    #[test]
    fn posting_without_choosing_both_accounts_sets_an_error() {
        let (ctx, cash, _capital) = ctx_with_two_accounts();
        let mut state = State {
            debit_account: Some(cash),
            ..Default::default()
        };

        update(&mut state, Message::Post, &ctx);

        assert_eq!(
            state.error.as_deref(),
            Some("choose both a debit and a credit account")
        );
    }

    #[test]
    fn posting_a_non_numeric_amount_sets_an_error() {
        let (ctx, cash, capital) = ctx_with_two_accounts();
        let mut state = State {
            debit_account: Some(cash),
            credit_account: Some(capital),
            amount_input: "not a number".into(),
            ..Default::default()
        };

        update(&mut state, Message::Post, &ctx);

        assert_eq!(state.error.as_deref(), Some("amount is not a valid number"));
    }

    #[test]
    fn posting_a_valid_entry_clears_the_form_and_reports_the_posted_transaction() {
        let (ctx, cash, capital) = ctx_with_two_accounts();
        let mut state = State {
            debit_account: Some(cash),
            credit_account: Some(capital),
            amount_input: "500".into(),
            memo_input: "Owner contribution".into(),
            ..Default::default()
        };

        update(&mut state, Message::Post, &ctx);

        assert!(state.error.is_none());
        assert!(state
            .message
            .as_deref()
            .unwrap_or_default()
            .starts_with("Posted transaction"));
        assert!(state.amount_input.is_empty());
        assert!(state.memo_input.is_empty());

        let trial_balance = AccountingRepository::new(ctx.db.clone())
            .trial_balance_as_of(ctx.company_id, date(2030, 1, 1))
            .unwrap();
        assert!(trial_balance.is_balanced());
    }
}
