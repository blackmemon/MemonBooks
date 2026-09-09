//! The landing screen after login: a snapshot of the company's books.

use chrono::Utc;
use iced::widget::{column, container, row, text};
use iced::{Element, Length};

use crate::backend::core::errors::AppResult;
use crate::backend::services::report_service::ReportService;
use crate::frontend::styles::colors::Theme;
use crate::frontend::styles::spacing;
use crate::frontend::styles::typography;
use crate::frontend::views::Ctx;

#[derive(Debug, Clone, Default)]
pub struct State {
    pub total_assets: String,
    pub total_liabilities: String,
    pub total_equity: String,
    pub open_receivables_count: usize,
    pub open_payables_count: usize,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    Refresh,
}

pub fn load(ctx: &Ctx) -> State {
    match load_summary(ctx) {
        Ok(state) => state,
        Err(err) => State {
            error: Some(err.to_string()),
            ..Default::default()
        },
    }
}

fn load_summary(ctx: &Ctx) -> AppResult<State> {
    let service = ReportService::new(ctx.db.clone());
    let today = Utc::now().date_naive();
    let trial_balance = service.trial_balance_as_of(ctx.company_id, today)?;

    Ok(State {
        total_assets: trial_balance.total_debits().to_string(),
        total_liabilities: String::new(),
        total_equity: trial_balance.total_credits().to_string(),
        open_receivables_count: service.list_open_receivables(ctx.company_id)?.len(),
        open_payables_count: service.list_open_payables(ctx.company_id)?.len(),
        error: None,
    })
}

pub fn update(state: &mut State, message: Message, ctx: &Ctx) {
    match message {
        Message::Refresh => *state = load(ctx),
    }
}

fn summary_card(theme: Theme, label: impl Into<String>, value: impl Into<String>) -> Element<'static, Message> {
    container(
        column![text(label.into()).size(typography::emphasis()).color(theme.primary()), text(value.into()).size(typography::stat_value()).color(theme.text_primary())].spacing(spacing::SPACE_6),
    )
    .padding(spacing::MD)
    .width(Length::FillPortion(1))
    .into()
}

pub fn view(state: &State, theme: Theme) -> Element<'_, Message> {
    if let Some(error) = &state.error {
        return container(text(format!("Could not load dashboard: {error}")).color(theme.negative_front()))
            .padding(spacing::SPACE_20)
            .into();
    }

    column![
        text("Dashboard").size(typography::title()).color(theme.text_primary()),
        row![
            summary_card(theme, "Total Debits (Trial Balance)", state.total_assets.clone()),
            summary_card(theme, "Total Credits (Trial Balance)", state.total_equity.clone()),
        ]
        .spacing(spacing::MD),
        row![
            summary_card(theme, "Open Receivables", state.open_receivables_count.to_string()),
            summary_card(theme, "Open Payables", state.open_payables_count.to_string()),
        ]
        .spacing(spacing::MD),
    ]
    .spacing(spacing::SPACE_20)
    .padding(spacing::SPACE_20)
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::accounting::accounting_rules::AccountType;
    use crate::backend::accounting::accounts_payable::payables::Payable;
    use crate::backend::accounting::accounts_receivable::receivables::Receivable;
    use crate::backend::accounting::chart_of_accounts::Account;
    use crate::backend::accounting::fiscal_periods::FiscalPeriod;
    use crate::backend::organization::companies::Company;
    use crate::backend::parties::customers::Customer;
    use crate::backend::parties::vendors::Vendor;
    use crate::backend::repositories::accounting_repository::{self, AccountingRepository};
    use crate::backend::repositories::organization_repository::{self, OrganizationRepository};
    use crate::backend::repositories::party_repository::{self, PartyRepository};
    use crate::backend::transactions::transaction::Transaction;
    use crate::backend::transactions::transaction_lines::TransactionLine;
    use crate::backend::transactions::transaction_types::TransactionType;
    use crate::database::database::Database;
    use chrono::NaiveDate;
    use rust_decimal_macros::dec;
    use std::path::PathBuf;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn ctx_with_ledger_activity() -> Ctx {
        let db = Database::open_in_memory().unwrap();
        db.run_migrations(organization_repository::MIGRATIONS).unwrap();
        db.run_migrations(party_repository::MIGRATIONS).unwrap();
        db.run_migrations(accounting_repository::MIGRATIONS).unwrap();
        db.run_migrations(crate::backend::audit::audit_logs::MIGRATIONS).unwrap();

        let org_repo = OrganizationRepository::new(db.clone());
        let company_id = org_repo.create_company(&Company::new("Acme Trading Co", "USD")).unwrap().id.unwrap();

        let party_repo = PartyRepository::new(db.clone());
        let customer_id = party_repo.create_customer(&Customer::new(company_id, "Acme Retail")).unwrap().id.unwrap();
        let vendor_id = party_repo.create_vendor(&Vendor::new(company_id, "Acme Supplies")).unwrap().id.unwrap();

        let accounting_repo = AccountingRepository::new(db.clone());
        accounting_repo.create_fiscal_period(&FiscalPeriod::new(company_id, "Open Period", date(2026, 1, 1), date(2030, 12, 31))).unwrap();
        let cash = accounting_repo.create_account(&Account::new(company_id, "1000", "Cash", AccountType::Asset)).unwrap();
        let ar = accounting_repo.create_account(&Account::new(company_id, "1100", "Accounts Receivable", AccountType::Asset)).unwrap();
        let ap = accounting_repo.create_account(&Account::new(company_id, "2000", "Accounts Payable", AccountType::Liability)).unwrap();
        let revenue = accounting_repo.create_account(&Account::new(company_id, "4000", "Sales Revenue", AccountType::Revenue)).unwrap();

        accounting_repo
            .post_transaction(
                &Transaction::new(company_id, TransactionType::OpeningBalance, date(2026, 1, 1))
                    .with_line(TransactionLine::debit(cash.id.unwrap(), dec!(5000)))
                    .with_line(TransactionLine::credit(ar.id.unwrap(), dec!(5000))),
            )
            .unwrap();

        accounting_repo
            .post_transaction_with_receivable(
                &Transaction::new(company_id, TransactionType::SalesInvoice, date(2026, 1, 5))
                    .with_line(TransactionLine::debit(ar.id.unwrap(), dec!(1000)))
                    .with_line(TransactionLine::credit(revenue.id.unwrap(), dec!(1000))),
                &Receivable::new(company_id, customer_id, date(2026, 1, 5), date(2026, 2, 4), dec!(1000)),
            )
            .unwrap();

        accounting_repo
            .post_transaction_with_payable(
                &Transaction::new(company_id, TransactionType::PurchaseBill, date(2026, 1, 6))
                    .with_line(TransactionLine::debit(cash.id.unwrap(), dec!(300)))
                    .with_line(TransactionLine::credit(ap.id.unwrap(), dec!(300))),
                &Payable::new(company_id, vendor_id, date(2026, 1, 6), date(2026, 2, 5), dec!(300)),
            )
            .unwrap();

        Ctx { db, company_id, current_user_id: None, export_dir: PathBuf::from("."), uploads_dir: PathBuf::from(".") }
    }

    #[test]
    fn refresh_loads_the_trial_balance_totals_and_open_item_counts() {
        let ctx = ctx_with_ledger_activity();
        let mut state = State::default();

        update(&mut state, Message::Refresh, &ctx);

        assert!(state.error.is_none());
        assert_eq!(state.open_receivables_count, 1);
        assert_eq!(state.open_payables_count, 1);
        assert_eq!(state.total_assets, state.total_equity); // trial balance stays balanced
    }
}
